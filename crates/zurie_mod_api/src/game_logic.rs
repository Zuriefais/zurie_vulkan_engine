use zurie_types::{glam::Vec2, Object};

use crate::utils::{get_obj_from_mem, obj_to_pointer};

#[derive(Default, Clone, Copy)]
pub struct ObjectHandle(u32);

impl ObjectHandle {
    pub fn get_pos(&self) -> Vec2 {
        get_object_position(self.0).unwrap()
    }

    pub fn set_pos(&self, pos: Vec2) {
        set_object_position(self.0, pos);
    }

    pub fn get_object(&self) -> Object {
        get_object(self.0).unwrap()
    }
}

pub fn spawn_object(obj: Object) -> ObjectHandle {
    let (ptr, len) = obj_to_pointer(&obj);
    ObjectHandle(unsafe { spawn_object_sys(ptr, len) })
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
