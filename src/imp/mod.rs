pub mod vulkan;

use std::{ffi::CStr, path::Path};

use ash::extensions::khr::ExternalMemoryFd;
use wgpu::{Adapter, DeviceDescriptor, RequestDeviceError};
use wgpu_hal::{api::Vulkan, InstanceDescriptor, InstanceFlags};

use crate::{
    imp::vulkan::{ash_update::ImageDrmFormatModifier, VulkanAdapterExt},
    AdapterExt, ExternalMemoryCapabilities, ExternalMemoryDevice, ExternalMemoryType,
    InstanceError,
};

#[derive(Debug)]
pub enum DeviceInner {
    Vulkan(vulkan::Inner),
}

impl AdapterExt for Adapter {
    fn request_device_with_external_memory(
        &self,
        desc: DeviceDescriptor,
        capabilities: ExternalMemoryCapabilities,
        trace_path: Option<&Path>,
    ) -> Result<(ExternalMemoryDevice, wgpu::Queue), RequestDeviceError> {
        #[cfg(vulkan)]
        {
            let is_vulkan = unsafe { self.as_hal::<Vulkan, _, bool>(|adapter| adapter.is_some()) };

            if is_vulkan {
                return vulkan::request_device(self, desc, trace_path);
            }
        }

        Err(RequestDeviceError)
    }

    fn external_memory_capabilities(
        &self,
        external_memory_type: ExternalMemoryType,
    ) -> ExternalMemoryCapabilities {
        #[cfg(vulkan)]
        {
            let is_vulkan = unsafe { self.as_hal::<Vulkan, _, bool>(|adapter| adapter.is_some()) };

            if is_vulkan {
                return vulkan::external_memory_capabilities(self, external_memory_type);
            }
        }

        ExternalMemoryCapabilities::empty()
    }
}

impl From<wgpu_hal::InstanceError> for InstanceError {
    fn from(_: wgpu_hal::InstanceError) -> Self {
        Self
    }
}

fn instance_desc() -> InstanceDescriptor<'static> {
    let mut flags = InstanceFlags::empty();
    if cfg!(debug_assertions) {
        flags |= InstanceFlags::VALIDATION;
        flags |= InstanceFlags::DEBUG;
    }
    InstanceDescriptor {
        name: "wgpu-external-memory",
        flags,
    }
}
