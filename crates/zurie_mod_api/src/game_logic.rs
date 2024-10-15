use zurie_types::{glam::Vec2, Object};

use crate::utils::{get_obj_from_mem, obj_to_pointer};

pub fn spawn_object(obj: Object) -> u32 {
    let (ptr, len) = obj_to_pointer(&obj);
    unsafe { spawn_object_sys(ptr, len) }
}

pub fn set_object_position(index: u32, position: Vec2) {
    let (ptr, len) = obj_to_pointer(&position);
    unsafe {
        set_object_position_sys(index, ptr, len);
    }
}

pub fn get_object_position(index: u32) -> Option<Vec2> {
    unsafe {
        if request_object_position_sys(index) {
            Some(get_obj_from_mem())
        } else {
            None
        }
    }
}

pub fn get_object(index: u32) -> Option<Object> {
    unsafe {
        if request_object_sys(index) {
            Some(get_obj_from_mem())
        } else {
            None
        }
    }
}

extern "C" {
    fn set_object_position_sys(index: u32, ptr: u32, len: u32);
    fn spawn_object_sys(ptr: u32, len: u32) -> u32;
    fn request_object_position_sys(index: u32) -> bool;
    fn request_object_sys(index: u32) -> bool;
}