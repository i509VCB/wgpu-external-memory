use wgpu::Instance;
use wgpu_hal::{InstanceDescriptor, InstanceFlags};

use crate::imp::{egl, vulkan};

/// Extension trait for creating an [`Instance`] that supports adapters that support DRM extensions.
pub trait InstanceExt: Sized {
    /// Create an new instance of wgpu capable of creating adapters that support DRM extensions.
    fn with_drm() -> Instance;
}

impl InstanceExt for Instance {
    fn with_drm() -> Instance {
        // Create the wgpu_core::Instance with Vulkan and EGL
        let instance = wgpu_core::instance::Instance {
            name: "wgpu-drm".into(),
            vulkan: vulkan::try_create_instance(INSTANCE_DESC),
            gl: egl::try_create_instance(INSTANCE_DESC),
        };

        // SAFETY: We initialized the instances ourselves and any hal backend safety requirements have been satisfied.
        unsafe { Instance::from_core(instance) }
    }
}

const INSTANCE_DESC: InstanceDescriptor = {
    let mut flags = InstanceFlags::empty();

    if cfg!(debug_assertions) {
        // impl const for traits is not stable. To make this const, convert the constants to bits and the
        // flags from bits.
        flags = InstanceFlags::from_bits_truncate(
            InstanceFlags::VALIDATION.bits() | InstanceFlags::DEBUG.bits(),
        );
    }

    InstanceDescriptor {
        name: "wgpu-drm",
        flags,
    }
};
