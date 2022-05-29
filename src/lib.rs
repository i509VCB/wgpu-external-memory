mod imp;

pub mod reexports {
    pub use wgpu;
    pub use wgpu_hal;
}

#[cfg(egl)]
pub mod egl;
// #[cfg(vulkan)]
pub mod vulkan;

use std::path::Path;

use bitflags::bitflags;
use imp::DeviceInner;
use wgpu::{DeviceDescriptor, RequestDeviceError};

#[derive(Debug)]
pub enum ExternalMemoryType {
    /// Linux Dmabuf handle.
    Dmabuf,
    // TODO: Other handle types
}

bitflags! {
    /// Describes what operations may be performed on external memory.
    pub struct ExternalMemoryCapabilities: u16 {
        /// Memory import is supported.
        const IMPORT = 0b0001;

        /// Memory export is supported.
        const EXPORT = 0b0010;

        /// Memory import and export is supported.
        const IMPORT_EXPORT = 0b0011;
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Not supported")]
pub struct InstanceError;

/// Extension trait to create a device which may import and export external memory handles.
pub trait AdapterExt: Sized {
    /// Requests a connection to a physical device, creating a logical device.
    ///
    /// Returns the Device together with a Queue that executes command buffers.
    ///
    /// This function is equivalent to [`Adapter::request_device`]. The returned device is capable of
    /// importing and exporting external memory handles.
    ///
    /// # Panics
    /// If the instance was not created using one of the extension functions defined in [`InstanceExt`].
    ///
    /// [`Adapter::request_device`]: wgpu::Adapter::request_device
    fn request_device_with_external_memory(
        &self,
        desc: DeviceDescriptor,
        trace_path: Option<&Path>,
    ) -> Result<(ExternalMemoryDevice, wgpu::Queue), RequestDeviceError>;

    /// Queries whether the specified type of external memory is supported.
    ///
    /// This can be used to filter out adapters which may not support the memory handles you require.
    fn supports_memory_type(&self, external_memory_type: ExternalMemoryType) -> bool;
}

/// A device capable of importing and exporting external memory objects.
pub struct ExternalMemoryDevice {
    device: wgpu::Device,
    inner: DeviceInner,
}

impl ExternalMemoryDevice {
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
}

impl AsRef<wgpu::Device> for ExternalMemoryDevice {
    fn as_ref(&self) -> &wgpu::Device {
        &self.device
    }
}

impl ExternalMemoryDevice {}
