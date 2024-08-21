use glam::{Mat4, Vec2, Vec4};
use log::info;
use winit::event::{KeyEvent, MouseScrollDelta, WindowEvent};

use crate::pixels_draw;

pub struct Camera {
    pub uniform: pixels_draw::vs::Camera,
    pub right: f32,
    pub left: f32,
    pub top: f32,
    pub bottom: f32,
    pub near: f32,
    pub far: f32,
    pub zoom_factor: f32,
    pub position: Vec2,
}

impl Camera {
    pub fn new(
        right: f32,
        left: f32,
        top: f32,
        bottom: f32,
        near: f32,
        far: f32,
        mut zoom_factor: f32,
        position: Vec2,
    ) -> Self {
        if zoom_factor == 0.0 {
            zoom_factor = 1.0;
        }

        Self {
            uniform: pixels_draw::vs::Camera {
                proj_mat: Mat4::ZERO.to_cols_array_2d(),
                cam_pos: (position / zoom_factor).into(),
            },
            right,
            left,
            top,
            bottom,
            near,
            far,
            zoom_factor,
            position,
        }
    }

    pub fn create_matrix(&self) -> Mat4 {
        let mut zoom_factor = self.zoom_factor;
        if zoom_factor == 0.0 {
            zoom_factor = 1.0;
        }

        let adjusted_left = self.left + (self.left * zoom_factor);
        let adjusted_right = self.right + (self.right * zoom_factor);
        let adjusted_bottom = self.bottom + (self.bottom * zoom_factor);
        let adjusted_top = self.top + (self.top * zoom_factor);

        Mat4::orthographic_rh(
            adjusted_left,
            adjusted_right,
            adjusted_bottom,
            adjusted_top,
            self.near,
            self.far,
        )
    }

    pub fn create_camera_from_screen_size(
        width: f32,
        height: f32,
        near: f32,
        far: f32,
        zoom_factor: f32,
        position: Vec2,
    ) -> Camera {
        let aspect = width / height;
        let left = -aspect / 2.0;
        let right = aspect / 2.0;
        let bottom = -0.5;
        let top = 0.5;
        Camera::new(right, left, top, bottom, near, far, zoom_factor, position)
    }

    pub fn update_matrix_from_screen_size(&mut self, width: f32, height: f32) {
        let aspect = width / height;
        let left = -aspect / 2.0;
        let right = aspect / 2.0;
        let bottom = -0.5;
        let top = 0.5;

        self.right = right;
        self.left = left;
        self.bottom = bottom;
        self.top = top;
        self.update_matrix();
    }

    pub fn update_matrix(&mut self) {
        self.uniform = pixels_draw::vs::Camera {
            proj_mat: self.create_matrix().to_cols_array_2d(),
            cam_pos: (self.position / self.zoom_factor).into(),
        }
    }

    pub fn get_matrix(&self) -> Mat4 {
        self.create_matrix()
    }

    pub fn event(&mut self, ev: WindowEvent) {
        match ev {
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(_, y) => {
                    if y > 0.0 && self.zoom_factor > 1.0 {
                        self.zoom_factor -= 0.5;
                    }
                    if y < 1.0 {
                        self.zoom_factor += 0.5;
                    }
                    self.update_matrix();
                    info!("Mouse scroll: {}, Zoom factor: {}", y, self.zoom_factor);
                }
                _ => {}
            },
            _ => {}
        }
    }
}
