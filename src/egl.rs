use wgpu::Instance;

use crate::InstanceError;

pub trait EglInstanceExt {
    /// Creates an Gles backed [`Instance`] that may be used to create devices that can import and export
    /// external memory handles.
    ///
    /// ## Platform-specific
    ///
    /// ### Linux
    ///
    /// Not implemented yet.
    ///
    /// Requires `EGL_EXT_image_dma_buf_import` for import.
    /// Requires `EGL_MESA_image_dma_buf_export` for export.
    ///
    /// ### Android
    ///
    /// Not implemented yet.
    ///
    /// Unsure how to implement. See `EGL_ANDROID_create_native_client_buffer` and
    /// `EGL_ANDROID_get_native_client_buffer`.
    ///
    /// ### Windows, macOS, iOS
    ///
    /// Unsupported.
    #[cfg(egl)]
    fn egl_with_external_memory() -> Result<Instance, InstanceError>;
}
