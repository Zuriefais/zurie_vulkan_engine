use glam::Vec2;
use log::info;
use winit::event::{ElementState, MouseButton, WindowEvent};

#[derive(Default)]
pub struct InputState {
    pub mouse: MouseState,
    pub keyboard: KeyboardState,
}

impl InputState {
    pub fn event(&mut self, ev: WindowEvent) {
        self.mouse.event(ev);
    }
}

#[derive(Default)]
pub struct KeyboardState {}

#[derive(Default)]
pub struct MouseState {
    pub position: Vec2,
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub hover_gui: bool,
}

impl MouseState {
    pub fn event(&mut self, ev: WindowEvent) {
        match ev {
            WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                (ElementState::Pressed, MouseButton::Left) => {
                    self.left_pressed = true;
                    info!("mouse pressed");
                }
                (ElementState::Released, MouseButton::Left) => {
                    self.left_pressed = false;
                    info!("mouse released");
                }
                (ElementState::Pressed, MouseButton::Right) => {
                    self.right_pressed = true;
                    info!("mouse pressed");
                }
                (ElementState::Released, MouseButton::Right) => {
                    self.right_pressed = false;
                    info!("mouse released");
                }
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.position = Vec2::new(position.x as f32, position.y as f32)
            }
            _ => {}
        }
    }
}
