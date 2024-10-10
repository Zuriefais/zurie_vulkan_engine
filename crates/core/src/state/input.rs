use hashbrown::HashSet;
use log::info;
use shared_types::{glam::Vec2, KeyCode};
use std::sync::{Arc, RwLock};
use winit::event::{ElementState, MouseButton, WindowEvent};

#[derive(Default)]
pub struct InputState {
    pub mouse: MouseState,
    pub keyboard: KeyboardState,
    pub pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
}

impl InputState {
    pub fn event(&mut self, ev: WindowEvent) {
        if let WindowEvent::KeyboardInput { event, .. } = ev.clone() {
            match event.physical_key {
                winit::keyboard::PhysicalKey::Code(key_code) => {
                    let key_code = key_code as u32;
                    let key_code: KeyCode = KeyCode::try_from(key_code).unwrap();
                    let mut keys_lock = self.pressed_keys_buffer.write().unwrap();
                    keys_lock.insert(key_code);
                }
                winit::keyboard::PhysicalKey::Unidentified(_) => {}
            }
        }
        self.mouse.event(ev);
    }

    pub fn after_update(&mut self) {
        if !self.pressed_keys_buffer.read().unwrap().is_empty() {
            let mut keys_lock = self.pressed_keys_buffer.write().unwrap();
            *keys_lock = HashSet::new();
        }
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
