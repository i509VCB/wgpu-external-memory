// TODO: Redesign as Linux specific extensions for wgpu
//
// # Vulkan & EGL
// - Dmabuf texture import/export
// - Identifying an adapter via the drm node's major and minor.
//
// # Vulkan
// - Fd memory import?
//
// # EGL
// - GBM Platform?
//
// Goals:
// - Allow sharing wgpu textures with other processes and graphics APIs.
// - Provide enough API to use wgpu in contexts where KMS is needed.
//
// Q?:
// - Creating textures with specific drm formats?

mod imp;

pub mod reexports {
    pub use wgpu;
    pub use wgpu_hal;
}

pub mod adapter;
pub mod instance;

use bitflags::bitflags;
use imp::DeviceInner;

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

/// A device capable of importing and exporting external memory objects.
#[derive(Debug)]
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
