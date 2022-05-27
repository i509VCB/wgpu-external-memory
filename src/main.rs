use wgpu::{DeviceDescriptor, DeviceType, Instance};
use wgpu_external_memory::{vulkan::VulkanInstanceExt, AdapterExt, ExternalMemoryCapabilities};
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

    let adapter = instance
        .enumerate_adapters(wgpu::Backends::VULKAN)
        .filter(|adapter| unsafe { adapter.as_hal::<Vulkan, _, _>(|adapter| adapter.is_some()) })
        .filter(|adapter| adapter.get_info().device_type == DeviceType::DiscreteGpu)
        .collect::<Vec<_>>()
        .into_iter()
        .next()
        .expect("No device?");

    let (device, _queue) = adapter
        .request_device_with_external_memory(
            DeviceDescriptor::default(),
            ExternalMemoryCapabilities::empty(),
            None,
        )
        .expect("Create device");

    let limits = device.as_ref().limits();
    unsafe {
        device.as_ref().as_hal::<Vulkan, _, _>(|device| {
            let device = device.unwrap();
            let enabled_extensions = device.enabled_device_extensions();
            dbg!(enabled_extensions);
        })
    }
}
