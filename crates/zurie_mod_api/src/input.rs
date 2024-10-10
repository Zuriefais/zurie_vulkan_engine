use zurie_types::{bitcode, glam::Vec2, KeyCode};

use crate::{LEN, PTR};

pub fn subscribe_for_key_event(key: KeyCode) {
    unsafe {
        subscribe_for_key_event_sys(key);
    }
}

extern "C" {
    fn subscribe_for_key_event_sys(key: KeyCode);
}

pub fn key_presed(key: KeyCode) -> bool {
    unsafe { key_pressed_sys(key) != 0 }
}

extern "C" {
    fn key_pressed_sys(key: KeyCode) -> i32;
}

pub fn get_mouse_pos() -> Vec2 {
    unsafe { request_mouse_pos_sys() };
    let data = unsafe { Vec::from_raw_parts(PTR as *mut u8, LEN as usize, LEN as usize) };
    let pos = bitcode::decode(&data).unwrap();
    std::mem::drop(data);
    pos
}

extern "C" {
    fn request_mouse_pos_sys();
}

// pub fn get_mouse_pos_in_world() -> Vec2 {}
