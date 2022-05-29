pub mod vulkan;

use std::path::Path;

use wgpu::{Adapter, DeviceDescriptor, RequestDeviceError};
use wgpu_hal::{api::Vulkan, InstanceDescriptor, InstanceFlags};

use crate::{AdapterExt, ExternalMemoryDevice, ExternalMemoryType, InstanceError};

#[derive(Debug)]
pub enum DeviceInner {
    Vulkan(vulkan::Inner),
}

impl AdapterExt for Adapter {
    fn request_device_with_external_memory(
        &self,
        desc: DeviceDescriptor,
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

    fn supports_memory_type(&self, external_memory_type: ExternalMemoryType) -> bool {
        #[cfg(vulkan)]
        {
            let is_vulkan = unsafe { self.as_hal::<Vulkan, _, bool>(|adapter| adapter.is_some()) };

            if is_vulkan {
                return vulkan::supports_memory_type(self, external_memory_type);
            }
        }

        false
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
