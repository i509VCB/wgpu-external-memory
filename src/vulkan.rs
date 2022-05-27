use wgpu::Instance;

use crate::InstanceError;

pub trait VulkanInstanceExt: Sized {
    /// Creates a Vulkan backed [`Instance`] that may be used to create devices that can import and export
    /// external memory handles.
    ///
    /// ## Platform-specific
    ///
    /// ### Linux
    ///
    /// Devices require `VK_EXT_image_drm_format_modifier` and `VK_EXT_external_memory_dma_buf`.
    ///
    /// ### Android
    ///
    /// Not implemented yet.
    ///
    /// Devices require `VK_ANDROID_external_memory_android_hardware_buffer`.
    ///
    /// ### Windows
    ///
    /// Not implemented yet.
    ///
    /// Devices require `VK_KHR_external_memory_win32`.
    ///
    /// ### macOS
    ///
    /// Unsupported.
    fn vulkan_with_external_memory() -> Result<Instance, InstanceError>;
}
