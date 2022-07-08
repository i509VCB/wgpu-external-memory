use wgpu::{Backends, Instance};

fn main() {
    let instance = Instance::new(Backends::all());
    let adapters = instance.enumerate_adapters(Backends::all());

    for adapter in adapters {
        // Get the hal representation, don't actually care if we have the right type.
        unsafe { adapter.as_hal::<wgpu_hal::api::Vulkan, _, _>(|_| ()) };
    }
}
