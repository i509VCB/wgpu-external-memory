use wgpu::{Backends, Instance};
use wgpu_drm::{adapter::AdapterExt, instance::InstanceExt};

fn main() {
    env_logger::init();

    let instance = Instance::with_drm();

    for (idx, adapter) in instance.enumerate_adapters(Backends::all()).enumerate() {
        let info = adapter.get_info();
        println!("Adapter {}: {}", idx, info.name);
        println!("\tBackend: {:?}", info.backend);
        println!("\tType: {:?}", info.device_type);

        if let Some(drm_info) = adapter.drm_info() {
            if let Some((major, minor)) = drm_info.primary_node {
                println!("\tPrimary node: {}:{}", major, minor);
            }

            if let Some((major, minor)) = drm_info.render_node {
                println!("\tRender node: {}:{}", major, minor);
            }
        }

        if let Some(uuids) = adapter.uuids() {
            println!("\tDevice UUID: {:?}", uuids.device_uuid);
            println!("\tDriver UUID: {:?}", uuids.driver_uuid);
        }
    }
}
