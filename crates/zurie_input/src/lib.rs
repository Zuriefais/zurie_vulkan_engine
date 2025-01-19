#![feature(fn_traits)]

use hashbrown::HashSet;
use log::info;
use std::sync::{Arc, RwLock};
use winit::event::{ElementState, MouseButton, WindowEvent};
use zurie_types::{KeyCode, glam::Vec2};

#[derive(Clone, Default)]
pub struct InputState {
    state: Arc<RwLock<InputStateInner>>,
}

impl InputState {
    pub fn event(&self, ev: WindowEvent) {
        self.get_inner_mut(|mut state| state.event(ev));
    }

    pub fn get_mouse_pos(&self) -> Vec2 {
        self.get_inner(|state| state.mouse.position)
    }

    pub fn left_mouse_button_pressed(&self) -> bool {
        self.get_inner(|state| state.mouse.left_pressed)
    }

    pub fn right_mouse_button_pressed(&self) -> bool {
        self.get_inner(|state| state.mouse.right_pressed)
    }

    pub fn pressed_keys(&self) -> HashSet<KeyCode> {
        self.get_inner(|state| state.pressed_keys_buffer.clone())
    }

    pub fn is_key_pressed(&self, key: &KeyCode) -> bool {
        self.get_inner(|state| state.pressed_keys_buffer.contains(key))
    }

    fn get_inner_mut<R>(&self, writer: impl FnOnce(&mut InputStateInner) -> R) -> R {
        writer.call_once((&mut self.state.write().unwrap(),))
    }

    fn get_inner<R>(&self, reader: impl FnOnce(&InputStateInner) -> R) -> R {
        reader.call_once((&self.state.read().unwrap(),))
    }

    pub fn after_update(&self) {
        self.get_inner_mut(|mut state| state.after_update());
    }
}

#[derive(Default, Clone)]
pub struct InputStateInner {
    pub mouse: MouseState,
    pub keyboard: KeyboardState,
    pub pressed_keys_buffer: HashSet<KeyCode>,
}

impl InputStateInner {
    pub fn event(&mut self, ev: WindowEvent) {
        if let WindowEvent::KeyboardInput { event, .. } = ev.clone() {
            match event.physical_key {
                winit::keyboard::PhysicalKey::Code(key_code) => {
                    let key_code = key_code as u32;
                    let key_code: KeyCode = KeyCode::try_from(key_code).unwrap();
                    self.pressed_keys_buffer.insert(key_code);
                }
                winit::keyboard::PhysicalKey::Unidentified(_) => {}
            }
        }
        self.mouse.event(ev);
    }

    pub fn after_update(&mut self) {
        if !self.pressed_keys_buffer.is_empty() {
            let mut keys_lock = self.pressed_keys_buffer.clear();
        }
    }
}

#[derive(Default, Clone)]
pub struct KeyboardState {}

#[derive(Default, Clone, Copy)]
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
                self.position = Vec2::new(position.x as f32, position.y as f32);
            }
            _ => {}
        }
    }
}
