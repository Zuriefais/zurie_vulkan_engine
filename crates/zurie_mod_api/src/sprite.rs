use zurie_types::glam::IVec2;

use crate::utils::string_to_pointer;

extern "C" {
    fn load_sprite_from_file_sys(path_ptr: u32, path_len: u32) -> u64;
    fn load_sprite_from_buffer_sys(buffer_ptr: u32, buffer_len: u32) -> u64;

    fn get_sprite_width_sys(handle: u64) -> i32;
    fn get_sprite_height_sys(handle: u64) -> i32;
}

pub fn get_sprite_dimensions(handle: u64) -> IVec2 {
    let (x, y) = unsafe { (get_sprite_width_sys(handle), get_sprite_height_sys(handle)) };
    IVec2 { x, y }
}

pub fn get_sprite_width(handle: u64) -> i32 {
    unsafe { get_sprite_width_sys(handle) }
}

pub fn get_sprite_height(handle: u64) -> i32 {
    unsafe { get_sprite_height_sys(handle) }
}

pub fn load_sprite_from_file(path: String) -> u64 {
    let (ptr, len) = string_to_pointer(path);
    unsafe { load_sprite_from_file_sys(ptr, len) }
}

pub fn load_sprite_from_buffer(buffer: &[u8]) -> u64 {
    let (ptr, len) = (buffer.as_ptr() as u32, buffer.len() as u32);
    unsafe { load_sprite_from_buffer_sys(ptr, len) }
}
