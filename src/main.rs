use wgpu::{DeviceDescriptor, Instance};
use wgpu_external_memory::{vulkan::VulkanInstanceExt, AdapterExt, ExternalMemoryType};
use wgpu_hal::api::Vulkan;

fn main() {
    env_logger::init();
    run();
}

fn run() {
    // let instance = wgpu::Instance::new(wgpu::Backends::VULKAN);
    let instance = Instance::vulkan_with_external_memory().expect("Create instance");

    let instance_extensions = unsafe {
        instance.as_hal::<Vulkan, _, _>(|instance| {
            Vec::from(instance.expect("Not Vulkan").extensions())
        })
    };

    dbg!(instance_extensions);

    let adapters = instance
        .enumerate_adapters(wgpu::Backends::VULKAN)
        // Filter out adapters that don't support external memory
        .filter(|adapter| {
            let supported = adapter.supports_memory_type(ExternalMemoryType::Dmabuf);

            if !supported {
                println!("{:?} does not support external memory", adapter.get_info());
            } else {
                println!("{:?} supports external memory", adapter.get_info());
            }

            supported
        });

    let adapter = adapters.into_iter().next().expect("no adapter?");

    let (device, _queue) = adapter
        .request_device_with_external_memory(DeviceDescriptor::default(), None)
        .expect("Create device");

    let _limits = device.as_ref().limits();
    unsafe {
        device.as_ref().as_hal::<Vulkan, _, _>(|device| {
            let device = device.unwrap();
            let enabled_extensions = device.enabled_device_extensions();
            dbg!(enabled_extensions);
        })
    }
}
