use std::path::Path;

use wgpu::{DeviceDescriptor, RequestDeviceError};

use crate::ExternalMemoryDevice;

/// Length of a UUID.
pub const UUID_LEN: usize = 16;

#[derive(Debug)]
#[non_exhaustive]
pub struct DrmInfo {
    /// The device major and minor numbers of this adapter's DRM device primary node.
    pub primary_node: Option<(u64, u64)>,

    /// The device major and minor numbers of this adapter's DRM device render node.
    pub render_node: Option<(u64, u64)>,
}

/// Persistent UUIDs that can identify a device and driver across graphics APIs.
///
/// - Vulkan: corresponds to values in `VkPhysicalDeviceIDProperties`.
/// - EGL: provided by `EGL_EXT_device_persistent_id` extension.
#[derive(Debug, Clone, Copy)]
pub struct DeviceUuids {
    /// The driver uuid of the adapter.
    ///
    /// - Vulkan: Equivalent to `driverUUID`
    /// - Gles: Equivalent to value of `DRIVER_UUID_EXT`
    pub driver_uuid: [u8; UUID_LEN],

    /// The device uuid of the adapter.
    ///
    /// - Vulkan: Equivalent to `deviceUUID`
    /// - Gles: Equivalent to value of `DEVICE_UUID_EXT`
    pub device_uuid: [u8; UUID_LEN],
}

/// Extension trait to create a device which may import and export external memory handles.
pub trait AdapterExt: Sized {
    /// Returns persistent uuids that may be used to identify a the adapter and driver.
    fn uuids(&self) -> Option<DeviceUuids>;

    /// Returns DRM specific information about the device.
    ///
    /// This will return [`None`] if any of the required extensions are not available.
    fn drm_info(&self) -> Option<DrmInfo>;

    fn supports_dmabuf_external_memory(&self) -> bool;

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
}
