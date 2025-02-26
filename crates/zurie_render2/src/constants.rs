use crate::debug::ValidationInfo;

use ash::vk; // Import the vk module from ash

use std::os::raw::c_char;

pub const APPLICATION_VERSION: u32 = vk::make_api_version(0, 1, 0, 0); // Variant, Major, Minor, Patch
pub const ENGINE_VERSION: u32 = vk::make_api_version(0, 1, 0, 0);
pub const API_VERSION: u32 = vk::make_api_version(0, 1, 0, 92);

pub const WINDOW_WIDTH: u32 = 800;
pub const WINDOW_HEIGHT: u32 = 600;
pub const VALIDATION: ValidationInfo = ValidationInfo {
    is_enable: true,
    required_validation_layers: ["VK_LAYER_KHRONOS_validation"],
};

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;
pub const IS_PAINT_FPS_COUNTER: bool = false;
