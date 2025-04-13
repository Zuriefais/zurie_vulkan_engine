use super::glam::{Mat4, Vec2};
use super::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub struct Camera {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
    pub zoom_factor: f32,
    pub position: Vec2,
    pub aspect_ratio: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            left: -1.0,
            right: 1.0,
            bottom: -1.0,
            top: 1.0,
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
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
        near: f32,
        far: f32,
        zoom_factor: f32,
        position: Vec2,
    ) -> Self {
        let zoom_factor = if zoom_factor <= 0.0 { 1.0 } else { zoom_factor };
        Self {
            left,
            right,
            top,
            bottom,
            near,
            far,
            zoom_factor,
            position,
            aspect_ratio: (right - left) / (top - bottom),
        }
    }

    pub fn from_screen_size(
        width: f32,
        height: f32,
        near: f32,
        far: f32,
        zoom_factor: f32,
        position: Vec2,
    ) -> Self {
        let aspect = width / height;
        let zoom_factor = if zoom_factor <= 0.0 { 1.0 } else { zoom_factor };
        Self {
            left: -aspect,
            right: aspect,
            bottom: -1.0,
            top: 1.0,
            near,
            far,
            zoom_factor,
            position,
            aspect_ratio: aspect,
        }
    }

    pub fn update_from_screen_size(&mut self, width: f32, height: f32) {
        self.aspect_ratio = width / height;
        self.left = -self.aspect_ratio;
        self.right = self.aspect_ratio;
        self.bottom = -1.0;
        self.top = 1.0;
    }

    pub fn create_matrix(&self) -> Mat4 {
        let zoom = self.zoom_factor;
        Mat4::orthographic_rh(
            self.left / zoom,
            self.right / zoom,
            self.bottom / zoom,
            self.top / zoom,
            self.near,
            self.far,
        ) * Mat4::from_translation(Vec2::new(-self.position.x, self.position.y).extend(0.0))
    }

    pub fn get_matrix(&self) -> Mat4 {
        self.create_matrix()
    }

    pub fn set_zoom(&mut self, zoom_factor: f32) {
        self.zoom_factor = if zoom_factor <= 0.0 { 1.0 } else { zoom_factor };
    }

    pub fn event(&mut self, scroll: f32) {
        let new_zoom = if scroll > 0.0 {
            (self.zoom_factor - 0.1).max(0.1)
        } else if scroll < 0.0 {
            self.zoom_factor + 0.1
        } else {
            self.zoom_factor
        };
        self.set_zoom(new_zoom);
    }
}
