use std::ffi::CStr;

use ash::{
    extensions::khr::ExternalMemoryFd,
    vk::{
        ExtExternalMemoryDmaBufFn, ExtImageDrmFormatModifierFn, KhrBindMemory2Fn,
        KhrGetMemoryRequirements2Fn, KhrImageFormatListFn, KhrMaintenance1Fn,
        KhrSamplerYcbcrConversionFn,
    },
};

#[allow(dead_code)]
pub const REQUIRED_DEVICE_EXTENSIONS: &[&CStr] = &[
    /* VK_EXT_external_memory_dma_buf */
    ExtExternalMemoryDmaBufFn::name(),
    ExternalMemoryFd::name(),
    /* VK_EXT_image_drm_format_modifier */
    ExtImageDrmFormatModifierFn::name(),
    KhrBindMemory2Fn::name(),            // or 1.1
    KhrImageFormatListFn::name(),        // or 1.2
    KhrSamplerYcbcrConversionFn::name(), // or 1.1
    KhrMaintenance1Fn::name(),           // or 1.1
    KhrGetMemoryRequirements2Fn::name(), // or 1.1
];
