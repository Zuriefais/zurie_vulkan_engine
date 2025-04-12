use std::sync::Arc;

use egui::Context;
use glam::{Vec2, Vec4};
use log::info;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};
use zurie_render_glue::{RenderBackend, RenderConfig};
use zurie_types::Object;

use crate::render_state::RenderState;

pub struct App {
    window: Option<Arc<Window>>,
    state: Option<RenderState>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: Default::default(),
            state: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("Creating window");
        if self.window.is_none() {
            let window_attributes =
                Window::default_attributes().with_title("Vulcan engine by Zuriefais");
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());
            let egui_context = Context::default();
            egui_context.set_style(gruvbox_egui::gruvbox_dark_theme());
            let state = RenderState::init(RenderConfig {
                window: window.clone(),
                event_loop,
                egui_context,
            })
            .unwrap();
            self.state = Some(state);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = self.state.as_mut().unwrap();
        let _ = state.handle_window_event(&event);
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::Resized(size) => {
                let _ = state.resize_window((size.width, size.height));
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                // Update egui's scale factor immediately
                state.egui_ctx.set_pixels_per_point(scale_factor as f32);
                log::info!("Scale factor: {}", scale_factor);
            }
            WindowEvent::RedrawRequested => {
                let objects = vec![
                    Object::new(
                        Vec2::new(1.0, 0.0),
                        Vec2::new(1.0, 1.0),
                        Vec4::new(1.0, 0.0, 0.0, 1.0),
                        0,
                        0,
                    ),
                    Object::new(
                        Vec2::new(0.0, 1.0),
                        Vec2::new(2.0, 2.0),
                        Vec4::new(0.0, 1.0, 0.0, 1.0),
                        0,
                        1,
                    ),
                    Object::new(
                        Vec2::new(0.0, 0.0),
                        Vec2::new(1.0, 1.0),
                        Vec4::new(0.0, 0.0, 1.0, 1.0),
                        0,
                        0,
                    ),
                ];
                state
                    .render(Default::default(), objects.into_iter())
                    .unwrap();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}
