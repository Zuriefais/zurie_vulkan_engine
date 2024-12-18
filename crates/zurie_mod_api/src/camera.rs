use zurie_types::{camera::Camera, glam::Vec2, Vector2};

use crate::utils::info;
use crate::{
    info,
    utils::{get_obj_from_mem, obj_to_pointer},
};

pub fn move_camera(direction: Vec2) {
    let new_cam_pos = get_camera_position() + direction;

    set_camera_position(new_cam_pos);
    info!(
        "cam pos: {}, cam_pos_expected: {}",
        get_camera_position(),
        new_cam_pos
    );
}

pub fn get_camera() -> Camera {
    unsafe {
        request_camera_sys();
    }
    get_obj_from_mem()
}

pub fn set_camera(camera: Camera) {
    let (ptr, len) = obj_to_pointer(&camera);
    unsafe {
        set_camera_sys(ptr, len);
    }
}
pub fn set_zoom_factor(factor: f32) {
    unsafe {
        set_zoom_factor_sys(factor);
    }
}
pub fn get_zoom_factor() -> f32 {
    unsafe { get_zoom_factor_sys() }
}

pub fn set_camera_position(position: Vec2) {
    let (ptr, len) = obj_to_pointer(&(Into::<Vector2>::into(-position)));
    unsafe {
        set_camera_position_sys(ptr, len);
    }
}

pub fn get_camera_position() -> Vec2 {
    unsafe {
        request_camera_position_sys();
    }
    -Into::<Vec2>::into(get_obj_from_mem::<Vector2>())
}

extern "C" {
    fn request_camera_sys();
    fn set_camera_sys(ptr: u32, len: u32);
    fn set_zoom_factor_sys(factor: f32);
    fn get_zoom_factor_sys() -> f32;
    fn request_camera_position_sys();
    fn set_camera_position_sys(ptr: u32, len: u32);
}
