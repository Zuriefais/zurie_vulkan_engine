use zurie_types::{glam::Vec2, KeyCode, Vector2};

use crate::utils::get_obj_from_mem;

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
    Into::<Vec2>::into(get_obj_from_mem::<Vector2>())
}

extern "C" {
    fn request_mouse_pos_sys();
}

// pub fn get_mouse_pos_in_world() -> Vec2 {}
