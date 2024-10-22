use zurie_types::{glam::Vec2, Object};

use crate::utils::{get_obj_from_mem, obj_to_pointer};
use std::sync::atomic::*;
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct ObjectHandle {
    id: u64,
    is_valid: Arc<std::sync::atomic::AtomicBool>,
}

impl ObjectHandle {
    pub fn is_valid(&self) -> bool {
        self.is_valid.load(Ordering::Relaxed)
    }

    pub fn new(id: u64) -> ObjectHandle {
        ObjectHandle {
            id,
            is_valid: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }
    pub fn get_pos(&self) -> Option<Vec2> {
        if self.is_valid() {
            get_object_position(self.id)
        } else {
            None
        }
    }

    pub fn set_pos(&self, pos: Vec2) {
        if self.is_valid() {
            set_object_position(self.id, pos);
        }
    }

    pub fn get_object(&self) -> Option<Object> {
        if self.is_valid() {
            get_object(self.id)
        } else {
            None
        }
    }

    pub fn despawn(&self) {
        despawn_object(self.id);
        self.is_valid.store(false, Ordering::Relaxed);
    }
}

pub fn spawn_object(obj: Object) -> ObjectHandle {
    let (ptr, len) = obj_to_pointer(&obj);
    ObjectHandle::new(unsafe { spawn_object_sys(ptr, len) })
}

pub fn set_object_position(index: u64, position: Vec2) {
    let (ptr, len) = obj_to_pointer(&position);
    unsafe {
        set_object_position_sys(index, ptr, len);
    }
}

pub fn get_object_position(index: u64) -> Option<Vec2> {
    unsafe {
        if request_object_position_sys(index) {
            Some(get_obj_from_mem())
        } else {
            None
        }
    }
}

pub fn get_object(index: u64) -> Option<Object> {
    unsafe {
        if request_object_sys(index) {
            Some(get_obj_from_mem())
        } else {
            None
        }
    }
}

pub fn despawn_object(index: u64) {
    unsafe {
        despawn_object_sys(index);
    }
}

extern "C" {
    fn set_object_position_sys(index: u64, ptr: u32, len: u32);
    fn spawn_object_sys(ptr: u32, len: u32) -> u64;
    fn despawn_object_sys(index: u64);
    fn request_object_position_sys(index: u64) -> bool;
    fn request_object_sys(index: u64) -> bool;
}
