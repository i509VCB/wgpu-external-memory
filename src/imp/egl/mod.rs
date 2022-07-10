use std::{ffi::c_void, mem, os::raw::c_uint};

use wgpu_core::api::Gles;
use wgpu_hal::{Api, InstanceDescriptor};

use crate::adapter::{DeviceUuids, DrmInfo, UUID_LEN};

pub const EGL_EXTENSIONS: i32 = 0x3055;
pub const EGL_DEVICE_EXT: i32 = 0x322C;

pub const DEVICE_UUID_EXT: i32 = 0x9597;
pub const DRIVER_UUID_EXT: i32 = 0x9598;

pub fn try_create_instance(desc: InstanceDescriptor<'static>) -> Option<<Gles as Api>::Instance> {
    // EGL does not require any additional instance creation parameters, so we can use the default `init` function.
    unsafe { <<Gles as Api>::Instance as wgpu_hal::Instance<Gles>>::init(&desc) }.ok()
}

pub fn get_device_uuids(adapter: Option<&<Gles as Api>::Adapter>) -> Option<DeviceUuids> {
    let adapter = adapter.unwrap();
    let instance = adapter.adapter_context().egl_instance()?;

    // GL_EXT_external_objects is an OpenGL extension, to test support make the context current.
    {
        use glow::HasContext;
        let context = adapter.adapter_context().lock();

        // GL_EXT_external_objects is provided by one of two names.
        const GL_EXTENSIONS: &[&str] = &["GL_EXT_memory_object", "GL_EXT_memory_object"];

        let gl_extensions = context.supported_extensions();

        if GL_EXTENSIONS
            .iter()
            .any(|name| gl_extensions.iter().any(|ext| ext == name))
        {
            // Load GetUnsignedBytevEXT
            let get_unsigned_bytev_ext = {
                let fn_ = instance.get_proc_address("glGetUnsignedBytevEXT")?;
                unsafe { mem::transmute::<_, GlGetUnsignedBytevEXT>(fn_) }
            };

            let mut device_uuid = [0u8; UUID_LEN];
            let mut driver_uuid = [0u8; UUID_LEN];

            unsafe {
                get_unsigned_bytev_ext(DEVICE_UUID_EXT, device_uuid.as_mut_ptr());
                get_unsigned_bytev_ext(DRIVER_UUID_EXT, driver_uuid.as_mut_ptr());

                return Some(DeviceUuids {
                    driver_uuid,
                    device_uuid,
                });
            }
        }
    }

    None
}

pub fn get_drm_info(adapter: Option<&<Gles as Api>::Adapter>) -> Option<DrmInfo> {
    let adapter = adapter.unwrap();
    let _egl_device = get_egl_device(adapter)?;

    None // TODO
}

fn get_egl_extensions(adapter: &<Gles as Api>::Adapter) -> Option<Vec<String>> {
    let instance = adapter.adapter_context().egl_instance()?;

    // Passing None for display is intentional as it returns the EGL extensions rather than display extensions.
    let egl_extensions = instance.query_string(None, EGL_EXTENSIONS).ok()?;
    let egl_extensions = egl_extensions.to_string_lossy().into_owned();
    Some(
        egl_extensions
            .split_whitespace()
            .map(|s| s.to_owned())
            .collect::<Vec<_>>(),
    )
}

#[allow(dead_code)]
fn get_display_extensions(adapter: &<Gles as Api>::Adapter) -> Option<Vec<String>> {
    let instance = adapter.adapter_context().egl_instance()?;
    let display = *adapter.adapter_context().raw_display()?;

    let egl_extensions = instance.query_string(Some(display), EGL_EXTENSIONS).ok()?;
    let egl_extensions = egl_extensions.to_string_lossy().into_owned();
    Some(
        egl_extensions
            .split_whitespace()
            .map(|s| s.to_owned())
            .collect::<Vec<_>>(),
    )
}

fn get_egl_device(adapter: &<Gles as Api>::Adapter) -> Option<*mut c_void> {
    let instance = adapter.adapter_context().egl_instance()?;
    let display = *adapter.adapter_context().raw_display()?;

    let egl_extensions = get_egl_extensions(adapter)?;

    const REQUIRED_EXTENSIONS: &[&str] = &["EGL_EXT_device_base", "EGL_EXT_device_query"];

    if REQUIRED_EXTENSIONS
        .iter()
        .all(|req| egl_extensions.iter().any(|name| name == req))
    {
        // Load the function to get the EGLDevice
        let query_display_attrib_ext = {
            let fn_ = instance.get_proc_address("eglQueryDisplayAttribEXT")?;
            unsafe { mem::transmute::<_, EglQueryDisplayAttribEXT>(fn_) }
        };

        let mut device: isize = 0;

        // Get the EGLDevice corresponding to the display
        if unsafe {
            query_display_attrib_ext(display.as_ptr(), EGL_DEVICE_EXT, &mut device as *mut _)
        } != 1
        {
            // No device available, could be software EGL.
            return None;
        }

        // 0 is NO_DEVICE_EXT
        if device == 0 {
            return None;
        }

        return Some(device as *mut c_void);
    }

    None
}

type EglQueryDisplayAttribEXT = unsafe extern "C" fn(
    *mut c_void, // dpy
    i32,         // attribute
    *mut isize,  // value
) -> c_uint;

type GlGetUnsignedBytevEXT = unsafe extern "C" fn(
    i32,     // pname
    *mut u8, // data
);
