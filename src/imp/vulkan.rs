use std::{
    ffi::{CStr, CString},
    fmt,
    path::Path,
};

use ash::{
    extensions::khr::ExternalMemoryFd,
    vk::{
        self, ExtExternalMemoryDmaBufFn, ExtImageDrmFormatModifierFn, KhrBindMemory2Fn,
        KhrExternalMemoryFn, KhrGetMemoryRequirements2Fn, KhrImageFormatListFn, KhrMaintenance1Fn,
        KhrSamplerYcbcrConversionFn,
    },
};
use wgpu::{Adapter, DeviceDescriptor, Features, Limits, RequestDeviceError};
use wgpu_hal::{
    api::Vulkan, Api, DeviceError, InstanceDescriptor, InstanceError, InstanceFlags, OpenDevice,
    UpdateAfterBindTypes,
};

use crate::{
    vulkan::VulkanInstanceExt, ExternalMemoryCapabilities, ExternalMemoryDevice, ExternalMemoryType,
};

use self::ash_update::ImageDrmFormatModifier;

use super::{instance_desc, DeviceInner};

impl VulkanInstanceExt for wgpu::Instance {
    fn vulkan_with_external_memory() -> Result<wgpu::Instance, crate::InstanceError> {
        unsafe {
            let hal_instance = with_external_memory(&instance_desc())?;
            Ok(wgpu::Instance::from_hal::<Vulkan>(hal_instance))
        }
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

            let raw_instance = device.raw_instance();
            let external_memory_fd = ExternalMemoryFd::new(raw_instance, raw_device);
            let image_drm_format_modifier = ImageDrmFormatModifier::new(raw_instance, raw_device);

            DeviceInner::Vulkan(Inner {
                external_memory_fd,
                image_drm_format_modifier,
            })
        })
    };

    let device = ExternalMemoryDevice { device, inner };

    Ok((device, queue))
}

pub fn external_memory_capabilities(
    adapter: &Adapter,
    external_memory_type: ExternalMemoryType,
) -> ExternalMemoryCapabilities {
    unsafe {
        adapter.as_hal::<Vulkan, _, _>(|adapter| {
            let adapter = adapter.unwrap();

            // Test if required instance extensions are supported.
            {
                let instance_extensions = adapter.enabled_instance_extensions();
                if instance_extensions.iter().all(|&extension_name| {
                    EXTERNAL_MEMORY_INSTANCE_EXTENSIONS
                        .iter()
                        .any(|&name| extension_name == name)
                }) {
                    return ExternalMemoryCapabilities::empty();
                }
            }

            todo!()
        });
    }

    todo!()
}

const EXTERNAL_MEMORY_INSTANCE_EXTENSIONS: &[&'static CStr] = &[
    vk::KhrExternalMemoryCapabilitiesFn::name(), // For KhrExternalMemoryFn
    // WGPU does require KhrGetPhysicalDeviceProperties2Fn. Add it anyways to be complete.
    vk::KhrGetPhysicalDeviceProperties2Fn::name(),
];

const REQUIRED_DEVICE_EXTENSIONS: &[&'static CStr] = &[
    // VK_EXT_external_memory_dma_buf
    ExtExternalMemoryDmaBufFn::name(),
    ExternalMemoryFd::name(),
    KhrExternalMemoryFn::name(),
    // VK_EXT_image_drm_format_modifier
    ExtImageDrmFormatModifierFn::name(),
    KhrBindMemory2Fn::name(),
    KhrImageFormatListFn::name(),
    // Some implementations (namely v3dv) do not support this extension and therefore cannot advertise
    // VK_EXT_image_drm_format_modifier in Vulkan 1.0.
    //
    // In Vulkan 1.1 this was promoted to core but is optional functionality. This is why v3dv only
    // provides VK_EXT_image_drm_format_modifier in Vulkan 1.1 or greater
    KhrSamplerYcbcrConversionFn::name(),
    KhrMaintenance1Fn::name(),
    KhrBindMemory2Fn::name(),
    KhrGetMemoryRequirements2Fn::name(),
];

// Copied from wgpu_hal::vulkan::Instance::init
unsafe fn with_external_memory(
    desc: &InstanceDescriptor,
) -> Result<<Vulkan as Api>::Instance, InstanceError> {
    let entry = match ash::Entry::load() {
        Ok(entry) => entry,
        Err(err) => {
            log::info!("Missing Vulkan entry points: {:?}", err);
            return Err(InstanceError);
        }
    };
    let driver_api_version = match entry.try_enumerate_instance_version() {
        // Vulkan 1.1+
        Ok(Some(version)) => version,
        Ok(None) => vk::API_VERSION_1_0,
        Err(err) => {
            log::warn!("try_enumerate_instance_version: {:?}", err);
            return Err(InstanceError);
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

    let mut extensions = <Vulkan as Api>::Instance::required_extensions(&entry, desc.flags)?;

    extensions.extend(EXTERNAL_MEMORY_INSTANCE_EXTENSIONS);

    let instance_layers = entry.enumerate_instance_layer_properties().map_err(|e| {
        log::info!("enumerate_instance_layer_properties: {:?}", e);
        InstanceError
    })?;

    let nv_optimus_layer = CStr::from_bytes_with_nul(b"VK_LAYER_NV_optimus\0").unwrap();
    let has_nv_optimus = instance_layers
        .iter()
        .any(|inst_layer| CStr::from_ptr(inst_layer.layer_name.as_ptr()) == nv_optimus_layer);

    // Check requested layers against the available layers
    let layers = {
        let mut layers: Vec<&'static CStr> = Vec::new();
        if desc.flags.contains(InstanceFlags::VALIDATION) {
            layers.push(CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap());
        }

        // Only keep available layers.
        layers.retain(|&layer| {
            if instance_layers
                .iter()
                .any(|inst_layer| CStr::from_ptr(inst_layer.layer_name.as_ptr()) == layer)
            {
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

        entry.create_instance(&create_info, None).map_err(|e| {
            log::warn!("create_instance: {:?}", e);
            InstanceError
        })?
    };

    <Vulkan as Api>::Instance::from_raw(
        entry,
        vk_instance,
        driver_api_version,
        extensions,
        desc.flags,
        has_nv_optimus,
        Some(Box::new(())), // `Some` signals that wgpu-hal is in charge of destroying vk_instance
    )
}

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
        let instance_extensions = self.enabled_instance_extensions();
        let iter = EXTERNAL_MEMORY_INSTANCE_EXTENSIONS
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

        // Extensions for Linux dmabuf external memory

        enabled_extensions.extend(REQUIRED_DEVICE_EXTENSIONS);

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
            self.raw_instance()
                .create_device(self.raw_physical_device(), &info, None)?
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
}

impl fmt::Debug for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceInner").finish_non_exhaustive()
    }
}

/// Stuff we need an ash update where this was added.
#[allow(dead_code)]
pub mod ash_update {
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
