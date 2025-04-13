use super::glam::{Mat4, Vec2};
use super::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub struct Camera {
    pub right: f32,
    pub left: f32,
    pub top: f32,
    pub bottom: f32,
    pub near: f32,
    pub far: f32,
    pub zoom_factor: f32,
    pub position: Vec2,
    pub aspect_ratio: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            right: 1.0,
            left: 1.0,
            top: 1.0,
            bottom: 1.0,
            near: -1.0,
            far: 1.0,
            zoom_factor: 1.0,
            position: Vec2::ZERO,
            aspect_ratio: 1.0,
        }
    }
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
            right,
            left,
            top,
            bottom,
            near,
            far,
            zoom_factor,
            position: position.into(),
            aspect_ratio: 1.0,
        }
    }

    pub fn create_matrix(&self) -> Mat4 {
        Mat4::orthographic_rh(
            -self.aspect_ratio, // Left
            self.aspect_ratio,  // Right
            -1.0,               // Bottom
            1.0,                // Top
            -1.0,               // Near
            1.0,                // Far
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
        let left = -aspect;
        let right = aspect;
        let bottom = -1.0;
        let top = 1.0;
        Camera::new(right, left, top, bottom, near, far, zoom_factor, position)
    }

    pub fn update_matrix_from_screen_size(&mut self, width: f32, height: f32) {
        let aspect = width / height;
        let left = -aspect;
        let right = aspect;
        let bottom = -1.0;
        let top = 1.0;

        self.right = right;
        self.left = left;
        self.bottom = bottom;
        self.top = top;
        self.aspect_ratio = aspect;
        //self.update_matrix();
    }

    // pub fn update_matrix(&mut self) {
    //     self.uniform = pixels_draw::vs::Camera {
    //         proj_mat: self.create_matrix().to_cols_array_2d(),
    //         cam_pos: (self.position / self.zoom_factor).into(),
    //     }
    // }

    pub fn get_matrix(&self) -> Mat4 {
        self.create_matrix()
    }

    pub fn event(&mut self, scroll: f32) {
        // if let WindowEvent::MouseWheel { delta, .. } = ev {
        //     if let MouseScrollDelta::LineDelta(_, y) = delta {

        //         //self.update_matrix();
        //         info!("Mouse scroll: {}, Zoom factor: {}", y, self.zoom_factor);
        //     }
        // }
        if scroll > 0.0 && self.zoom_factor > 1.0 {
            self.zoom_factor -= 0.5;
        }
        if scroll < 1.0 {
            self.zoom_factor += 0.5;
        }
        //self.update_matrix();
    }
}
