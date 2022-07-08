pub mod egl;
pub mod vulkan;

use std::path::Path;

use wgpu::{Adapter, DeviceDescriptor, RequestDeviceError};
use wgpu_hal::{api::Gles, api::Vulkan};

use crate::{
    adapter::{AdapterExt, DeviceUuids, DrmInfo},
    ExternalMemoryDevice,
};

#[derive(Debug)]
pub enum DeviceInner {
    Vulkan(vulkan::Inner),
}

impl AdapterExt for Adapter {
    fn uuids(&self) -> Option<DeviceUuids> {
        #[cfg(vulkan)]
        {
            let is_vulkan = unsafe { self.as_hal::<Vulkan, _, bool>(|adapter| adapter.is_some()) };

            if is_vulkan {
                return unsafe { self.as_hal::<Vulkan, _, _>(vulkan::get_device_uuids) };
            }
        }

        #[cfg(egl)]
        {
            let is_gl = unsafe { self.as_hal::<Gles, _, bool>(|adapter| adapter.is_some()) };

            if is_gl {
                return unsafe { self.as_hal::<Gles, _, _>(egl::get_device_uuids) };
            }
        }

        None
    }

    fn drm_info(&self) -> Option<DrmInfo> {
        #[cfg(vulkan)]
        {
            let is_vulkan = unsafe { self.as_hal::<Vulkan, _, bool>(|adapter| adapter.is_some()) };

            if is_vulkan {
                return unsafe { self.as_hal::<Vulkan, _, _>(vulkan::get_drm_info) };
            }
        }

        #[cfg(egl)]
        {
            let is_gl = unsafe { self.as_hal::<Gles, _, bool>(|adapter| adapter.is_some()) };

            if is_gl {
                return unsafe { self.as_hal::<Gles, _, _>(egl::get_drm_info) };
            }
        }

        None
    }

    fn supports_dmabuf_external_memory(&self) -> bool {
        false // TODO
    }

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

        // TODO: EGL

        Err(RequestDeviceError)
    }
}
