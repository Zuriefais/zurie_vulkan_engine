use ash::vk;
use log::{error, info, warn};

use std::ffi::CStr;
use std::os::raw::c_void;
use std::ptr;

// 1. Add the explicit debug utils extension import
use ash::ext::debug_utils; // Changed from ash::extensions::ext::DebugUtils

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let types = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[general]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[performance]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[validation]",
        _ => "[unknown]",
    };
    let message = unsafe { CStr::from_ptr((*p_callback_data).p_message) };
    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            warn!(target: "vulkan", "{} {}", types, message.to_str().unwrap())
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!(target: "vulkan", "{} {}", types, message.to_str().unwrap())
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!(target: "vulkan", "{} {}", types, message.to_str().unwrap())
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!(target: "vulkan", "{} {}", types, message.to_str().unwrap())
        }

        _ => info!(target: "vulkan", "{} {}", types, message.to_str().unwrap()),
    };

    vk::FALSE
}

pub struct ValidationInfo {
    pub is_enable: bool,
    pub required_validation_layers: [&'static str; 1],
}

pub fn check_validation_layer_support(
    entry: &ash::Entry,
    required_validation_layers: &Vec<&str>,
) -> bool {
    let layer_properties = unsafe {
        entry
            .enumerate_instance_layer_properties()
            .expect("Failed to enumerate Instance Layers Properties")
    };
    if layer_properties.len() <= 0 {
        eprintln!("No available layers.");
        return false;
    }

    for required_layer_name in required_validation_layers.iter() {
        let mut is_layer_found = false;

        for layer_property in layer_properties.iter() {
            let test_layer_name = super::tools::vk_to_string(&layer_property.layer_name);
            if (*required_layer_name) == test_layer_name {
                is_layer_found = true;
                break;
            }
        }

        if !is_layer_found {
            return false;
        }
    }

    true
}

// 2. Update the return type to use the correct DebugUtils path
pub fn setup_debug_utils(
    is_enable_debug: bool,
    entry: &ash::Entry,
    instance: &ash::Instance,
) -> (debug_utils::Instance, vk::DebugUtilsMessengerEXT) {
    // Changed from ash::extensions::ext::DebugUtils
    let debug_utils_loader = debug_utils::Instance::new(entry, instance);

    if !is_enable_debug {
        (debug_utils_loader, vk::DebugUtilsMessengerEXT::null())
    } else {
        let messenger_ci = populate_debug_messenger_create_info();

        let utils_messenger = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&messenger_ci, None)
                .expect("Debug Utils Callback")
        };

        (debug_utils_loader, utils_messenger)
    }
}

// 3. Use builder pattern to avoid lifetime issues
pub fn populate_debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
    vk::DebugUtilsMessengerCreateInfoEXT {
        s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
        p_next: std::ptr::null(),
        flags: vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
            | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        pfn_user_callback: Some(vulkan_debug_utils_callback),
        p_user_data: std::ptr::null_mut(),
        _marker: std::marker::PhantomData,
    }
}
