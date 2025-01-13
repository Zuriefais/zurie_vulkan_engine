use zurie_types::Vector2;

use super::ScriptingState;
use crate::functions::zurie::engine::camera;

use crate::functions::zurie::engine::core::Vec2;
impl camera::Host for ScriptingState {
    fn get_camera(&mut self) -> camera::Camera {
        let camera = self.camera.read().unwrap();
        camera::Camera {
            position: Vec2 {
                x: camera.position.x,
                y: camera.position.y,
            },
            zoom_factor: camera.zoom_factor,
        }
    }

    fn set_camera(&mut self, camera: camera::Camera) -> () {
        let mut engine_camera = self.camera.write().unwrap();
        engine_camera.zoom_factor = camera.zoom_factor;
        engine_camera.position = Vector2 {
            x: camera.position.x,
            y: camera.position.y,
        };
    }

    fn set_zoom(&mut self, factor: f32) -> () {
        let mut engine_camera = self.camera.write().unwrap();
        engine_camera.zoom_factor = factor;
    }

    fn get_zoom(&mut self) -> f32 {
        self.camera.read().unwrap().zoom_factor
    }

    fn set_position(&mut self, position: Vec2) -> () {
        self.camera.write().unwrap().position = Vector2 {
            x: position.x,
            y: position.y,
        }
    }

    fn get_position(&mut self) -> Vec2 {
        let camera = self.camera.read().unwrap();

        Vec2 {
            x: camera.position.x,
            y: camera.position.y,
        }
    }
}
