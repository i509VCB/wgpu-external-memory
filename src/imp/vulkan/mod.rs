mod dmabuf;

use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    fmt,
    path::Path,
};

use ash::{
    extensions::khr::{ExternalMemoryFd, GetPhysicalDeviceProperties2},
    vk::{self, KhrExternalMemoryFn},
};
use drm_fourcc::{DrmFormat, DrmFourcc, DrmModifier};
use wgpu::{Adapter, DeviceDescriptor, Features, Limits, RequestDeviceError};
use wgpu_hal::{
    api::Vulkan, Api, DeviceError, InstanceDescriptor, InstanceError, InstanceFlags, OpenDevice,
    UpdateAfterBindTypes,
};

use crate::{
    adapter::{DeviceUuids, DrmInfo},
    ExternalMemoryDevice,
};

use self::ash_upstreamed::ImageDrmFormatModifier;

use super::DeviceInner;

pub fn try_create_instance(desc: InstanceDescriptor<'static>) -> Option<<Vulkan as Api>::Instance> {
    // Creating a Vulkan instance for wgpu is more complicated than EGL. Vulkan requires all extensions to be
    // explicitly enabled at creation time and specific extensions may require enabling specific Vulkan
    // features.
    //
    // Much of the code here is copied from wgpu_hal::vulkan::Instance::init

    const REQUIRED_INSTANCE_EXTENSIONS: &[&CStr] = &[
        // dependency of VK_KHR_external_memory
        //
        // This is required or Vulkan 1.1
        vk::KhrExternalMemoryCapabilitiesFn::name(),
        // WGPU requires VK_KHR_physical_device_properties2
        //
        // Listed for completeness
        vk::KhrGetPhysicalDeviceProperties2Fn::name(),
    ];

    let entry = match unsafe { ash::Entry::load() } {
        Ok(entry) => entry,
        Err(err) => {
            log::info!("Missing Vulkan entry points: {:?}", err);
            return None;
        }
    };
    let driver_api_version = match entry.try_enumerate_instance_version() {
        // Vulkan 1.1+
        Ok(Some(version)) => version,
        Ok(None) => vk::API_VERSION_1_0,
        Err(err) => {
            log::warn!("try_enumerate_instance_version: {:?}", err);
            return None;
        }
    };

    let app_name = CString::new(desc.name).unwrap();
    let app_info = vk::ApplicationInfo::builder()
        .application_name(app_name.as_c_str())
        .application_version(1)
        .engine_name(CStr::from_bytes_with_nul(b"wgpu-hal\0").unwrap())
        .engine_version(2)
        .api_version(
            // Vulkan 1.0 doesn't like anything but 1.0 passed in here...
            if driver_api_version < vk::API_VERSION_1_1 {
                vk::API_VERSION_1_0
            } else {
                // This is the max Vulkan API version supported by `wgpu-hal`.
                //
                // If we want to increment this, there are some things that must be done first:
                //  - Audit the behavioral differences between the previous and new API versions.
                //  - Audit all extensions used by this backend:
                //    - If any were promoted in the new API version and the behavior has changed, we must handle the new behavior in addition to the old behavior.
                //    - If any were obsoleted in the new API version, we must implement a fallback for the new API version
                //    - If any are non-KHR-vendored, we must ensure the new behavior is still correct (since backwards-compatibility is not guaranteed).
                vk::HEADER_VERSION_COMPLETE
            },
        );

    let mut extensions = <Vulkan as Api>::Instance::required_extensions(&entry, desc.flags).ok()?;
    extensions.extend(REQUIRED_INSTANCE_EXTENSIONS);

    let instance_layers = entry
        .enumerate_instance_layer_properties()
        .map_err(|e| {
            log::info!("enumerate_instance_layer_properties: {:?}", e);
            InstanceError
        })
        .ok()?;

    let nv_optimus_layer = CStr::from_bytes_with_nul(b"VK_LAYER_NV_optimus\0").unwrap();
    let has_nv_optimus = instance_layers.iter().any(
        |inst_layer| unsafe { CStr::from_ptr(inst_layer.layer_name.as_ptr()) } == nv_optimus_layer,
    );

    // Check requested layers against the available layers
    let layers = {
        let mut layers: Vec<&'static CStr> = Vec::new();
        if desc.flags.contains(InstanceFlags::VALIDATION) {
            layers.push(CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap());
        }

        // Only keep available layers.
        layers.retain(|&layer| {
            if instance_layers.iter().any(
                |inst_layer| unsafe { CStr::from_ptr(inst_layer.layer_name.as_ptr()) } == layer,
            ) {
                true
            } else {
                log::warn!("Unable to find layer: {}", layer.to_string_lossy());
                false
            }
        });
        layers
    };

    let vk_instance = {
        let str_pointers = layers
            .iter()
            .chain(extensions.iter())
            .map(|&s| {
                // Safe because `layers` and `extensions` entries have static lifetime.
                s.as_ptr()
            })
            .collect::<Vec<_>>();

        let create_info = vk::InstanceCreateInfo::builder()
            .flags(vk::InstanceCreateFlags::empty())
            .application_info(&app_info)
            .enabled_layer_names(&str_pointers[..layers.len()])
            .enabled_extension_names(&str_pointers[layers.len()..]);

        unsafe {
            entry.create_instance(&create_info, None).map_err(|e| {
                log::warn!("create_instance: {:?}", e);
                InstanceError
            })
        }
        .ok()?
    };

    unsafe {
        <Vulkan as Api>::Instance::from_raw(
            entry,
            vk_instance,
            driver_api_version,
            0, // Android SDK
            extensions,
            desc.flags,
            has_nv_optimus,
            Some(Box::new(())), // `Some` signals that wgpu-hal is in charge of destroying vk_instance
        )
    }
    .ok()
}

pub fn get_device_uuids(_adapter: Option<&<Vulkan as Api>::Adapter>) -> Option<DeviceUuids> {
    None // TODO
}

pub fn get_drm_info(adapter: Option<&<Vulkan as Api>::Adapter>) -> Option<DrmInfo> {
    let adapter = adapter.unwrap();
    let info = get_adapter_drm_info(adapter)?;

    let mut uuids = DrmInfo {
        primary_node: None,
        render_node: None,
    };

    if info.has_primary == vk::TRUE {
        uuids.primary_node = Some((info.primary_major as _, info.primary_minor as _));
    }

    if info.has_render == vk::TRUE {
        uuids.render_node = Some((info.render_major as _, info.render_minor as _));
    }

    Some(uuids)
}

fn get_adapter_drm_info(
    adapter: &<Vulkan as Api>::Adapter,
) -> Option<vk::PhysicalDeviceDrmPropertiesEXT> {
    let phd = adapter.raw_physical_device();

    if let Ok(extensions) = unsafe {
        adapter
            .shared_instance()
            .raw_instance()
            .enumerate_device_extension_properties(phd)
    } {
        if extensions.iter().any(|properties| {
            let name = unsafe { CStr::from_ptr(&properties.extension_name as *const _) };
            name == vk::ExtPhysicalDeviceDrmFn::name()
        }) {
            let shared = adapter.shared_instance();

            let mut physical_device_drm = vk::PhysicalDeviceDrmPropertiesEXT::builder();
            let mut properties =
                vk::PhysicalDeviceProperties2::builder().push_next(&mut physical_device_drm);

            // wgpu requires VK_KHR_get_physical_device_properties2, no need to check for the extension.
            unsafe {
                get_physical_device_properties2(
                    shared.entry(),
                    shared.raw_instance(),
                    phd,
                    &mut properties,
                    shared.driver_api_version(),
                )
            };

            return Some(physical_device_drm.build());
        }
    }

    // Device was lost or required extensions are not available.
    None
}

unsafe fn get_physical_device_properties2(
    entry: &ash::Entry,
    instance: &ash::Instance,
    phd: vk::PhysicalDevice,
    properties: &mut vk::PhysicalDeviceProperties2,
    version: u32,
) {
    if version > vk::API_VERSION_1_0 {
        instance.get_physical_device_properties2(phd, properties)
    } else {
        // Load the extension function
        let fns = GetPhysicalDeviceProperties2::new(entry, instance);
        fns.get_physical_device_properties2(phd, properties)
    }
}

pub fn request_device(
    adapter: &Adapter,
    desc: DeviceDescriptor,
    trace_path: Option<&Path>,
) -> Result<(ExternalMemoryDevice, wgpu::Queue), RequestDeviceError> {
    let hal_device = unsafe {
        adapter.as_hal::<Vulkan, _, _>(|adapter| {
            let adapter = adapter.unwrap();
            adapter.open_with_external_memory(desc.features, &desc.limits)
        })
    }
    .map_err(|_| RequestDeviceError)?;

    let (device, queue) = unsafe { adapter.create_device_from_hal(hal_device, &desc, trace_path) }?;

    let inner = unsafe {
        device.as_hal::<Vulkan, _, _>(|device| {
            let device = device.unwrap();
            let raw_device = device.raw_device();
            let raw_instance = device.shared_instance().raw_instance();

            DeviceInner::Vulkan(Inner::new(
                raw_instance,
                device.raw_physical_device(),
                raw_device,
            ))
        })
    };

    let device = ExternalMemoryDevice { device, inner };

    Ok((device, queue))
}

/// Instance extensions required to use external memory.
const REQUIRED_INSTANCE_EXTENSIONS: &[&CStr] = &[
    // dependency of VK_KHR_external_memory
    //
    // This is required or 1.1
    vk::KhrExternalMemoryCapabilitiesFn::name(),
    // WGPU requires VK_KHR_physical_device_properties2
    //
    // Listed for completeness
    vk::KhrGetPhysicalDeviceProperties2Fn::name(),
];

/// Device extensions required to use all external memory handles.
const REQUIRED_DEVICE_EXTENSIONS: &[&CStr] = &[
    KhrExternalMemoryFn::name(), // Or 1.1
];

pub trait VulkanAdapterExt: Sized {
    unsafe fn open_with_external_memory(
        &self,
        features: Features,
        limits: &Limits,
    ) -> Result<OpenDevice<Vulkan>, DeviceError>;
}

impl VulkanAdapterExt for <Vulkan as Api>::Adapter {
    unsafe fn open_with_external_memory(
        &self,
        features: Features,
        limits: &Limits,
    ) -> Result<OpenDevice<Vulkan>, DeviceError> {
        let phd_limits = self.physical_device_capabilities().properties().limits;
        let uab_types = UpdateAfterBindTypes::from_limits(limits, &phd_limits);
        let mut enabled_extensions = self.required_device_extensions(features);

        // Test that all required instance extensions are available
        let instance_extensions = self.shared_instance().extensions();
        let iter = REQUIRED_INSTANCE_EXTENSIONS
            .iter()
            .filter(|&&req_extension| {
                !instance_extensions
                    .iter()
                    .any(|&name| name == req_extension)
            })
            .collect::<Vec<_>>();

        if !iter.is_empty() {
            for &&missing in &iter {
                log::error!("Missing instance extension {:?}", missing);
            }

            panic!("Missing required instance extensions")
        }

        // Extensions for external memory
        enabled_extensions.extend(REQUIRED_DEVICE_EXTENSIONS);

        // TODO: All handle types
        // TODO: Enable dmabuf device extensions

        let mut enabled_phd_features =
            self.physical_device_features(&enabled_extensions, features, uab_types);

        let str_pointers = enabled_extensions
            .iter()
            .map(|&s| {
                // Safe because `enabled_extensions` entries have static lifetime.
                s.as_ptr()
            })
            .collect::<Vec<_>>();

        let family_index = 0; //TODO
        let family_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(family_index)
            .queue_priorities(&[1.0])
            .build();
        let family_infos = [family_info];

        let pre_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&family_infos)
            .enabled_extension_names(&str_pointers);
        let info = enabled_phd_features
            .add_to_device_create_builder(pre_info)
            .build();
        let raw_device = {
            // profiling::scope!("vkCreateDevice");
            self.shared_instance().raw_instance().create_device(
                self.raw_physical_device(),
                &info,
                None,
            )?
        };

        self.device_from_raw(
            raw_device,
            true,
            &enabled_extensions,
            features,
            uab_types,
            family_info.queue_family_index,
            0,
        )
    }
}

pub struct Inner {
    pub external_memory_fd: ExternalMemoryFd,
    pub image_drm_format_modifier: ImageDrmFormatModifier,
    pub supported_drm_formats: HashMap<DrmFormat, vk::DrmFormatModifierPropertiesEXT>,
}

impl Inner {
    pub fn new(instance: &ash::Instance, phd: vk::PhysicalDevice, device: &ash::Device) -> Self {
        let external_memory_fd = ExternalMemoryFd::new(instance, device);
        let image_drm_format_modifier = ImageDrmFormatModifier::new(instance, device);

        let mut supported_drm_formats = HashMap::new();

        // TODO: More formats
        {
            let modifier_properties =
                unsafe { get_drm_format_properties_list(instance, phd, vk::Format::B8G8R8A8_SRGB) };

            for properties in modifier_properties {
                let drm_format = DrmFormat {
                    code: DrmFourcc::Argb8888,
                    modifier: DrmModifier::from(properties.drm_format_modifier),
                };

                supported_drm_formats.insert(drm_format, properties);

                let drm_format = DrmFormat {
                    code: DrmFourcc::Xrgb8888,
                    modifier: DrmModifier::from(properties.drm_format_modifier),
                };

                supported_drm_formats.insert(drm_format, properties);
            }
        }

        Self {
            external_memory_fd,
            image_drm_format_modifier,
            supported_drm_formats,
        }
    }
}

impl fmt::Debug for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Inner")
            .field("supported_drm_formats", &self.supported_drm_formats)
            .finish()
    }
}

pub unsafe fn get_drm_format_properties_list(
    instance: &ash::Instance,
    pdevice: vk::PhysicalDevice,
    format: vk::Format,
) -> Vec<vk::DrmFormatModifierPropertiesEXT> {
    let mut list = vk::DrmFormatModifierPropertiesListEXT::default();

    // Need to get number of entries in list and then allocate the vector as needed.
    {
        let mut format_properties_2 = vk::FormatProperties2::builder().push_next(&mut list);

        instance.get_physical_device_format_properties2(pdevice, format, &mut format_properties_2);
    }

    let mut data = Vec::with_capacity(list.drm_format_modifier_count as usize);

    // Read the number of elements into the vector.
    list.p_drm_format_modifier_properties = data.as_mut_ptr();

    {
        let mut format_properties_2 = vk::FormatProperties2::builder().push_next(&mut list);

        instance.get_physical_device_format_properties2(pdevice, format, &mut format_properties_2);
    }

    data.set_len(list.drm_format_modifier_count as usize);
    data
}

/// This module contains code that has been upstreamed to ash, pending a new ash release and a wgpu update to
/// Vulkan.
#[allow(dead_code)]
pub mod ash_upstreamed {
    use ash::{prelude::*, vk, Device, Instance};
    use std::{ffi::CStr, mem};

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VK_EXT_image_drm_format_modifier.html>
    #[derive(Clone)]
    pub struct ImageDrmFormatModifier {
        handle: vk::Device,
        fp: vk::ExtImageDrmFormatModifierFn,
    }

    impl ImageDrmFormatModifier {
        pub fn new(instance: &Instance, device: &Device) -> Self {
            let handle = device.handle();
            let fp = vk::ExtImageDrmFormatModifierFn::load(|name| unsafe {
                mem::transmute(instance.get_device_proc_addr(handle, name.as_ptr()))
            });
            Self { handle, fp }
        }

        /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkGetImageDrmFormatModifierPropertiesEXT.html>
        pub unsafe fn get_image_drm_format_modifier_properties(
            &self,
            image: vk::Image,
            properties: &mut vk::ImageDrmFormatModifierPropertiesEXT,
        ) -> VkResult<()> {
            (self.fp.get_image_drm_format_modifier_properties_ext)(self.handle, image, properties)
                .result()
        }

        pub const fn name() -> &'static CStr {
            vk::ExtImageDrmFormatModifierFn::name()
        }

        pub fn fp(&self) -> &vk::ExtImageDrmFormatModifierFn {
            &self.fp
        }

        pub fn device(&self) -> vk::Device {
            self.handle
        }
    }
}
