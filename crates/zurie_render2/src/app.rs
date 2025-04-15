use std::sync::Arc;

use egui::{Context, ViewportId};
use egui_winit::State;
use glam::{Vec2, Vec4};
use log::info;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};
use zurie_render_glue::{FrameContext, RenderBackend, RenderConfig};
use zurie_types::{Object, camera::Camera};

use crate::render::renderer::Renderer;

pub struct App {
    window: Option<Arc<Window>>,
    state: Option<Renderer>,
    frame_context: FrameContext,
    egui_winit: Option<State>,
    egui_context: Option<Context>,
}

impl Default for App {
    fn default() -> Self {
        let mut frame_context: FrameContext = Default::default();
        frame_context.camera = Camera::default();
        frame_context.camera.position = Vec2::new(-1.0, 1.0);

        Self {
            window: Default::default(),
            state: None,
            frame_context,
            egui_winit: None,
            egui_context: None,
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
            let state = Renderer::init(RenderConfig {
                window: window.clone(),
                event_loop,
                egui_context: egui_context.clone(),
            })
            .unwrap();
            let egui_winit = Some(State::new(
                egui_context.clone(),
                ViewportId::ROOT,
                &window,
                None,
                None,
                None,
            ));
            self.state = Some(state);
            self.egui_winit = egui_winit;
            self.egui_context = Some(egui_context)
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
        self.egui_winit
            .as_mut()
            .unwrap()
            .on_window_event(&self.window.as_ref().unwrap(), &event);
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
                self.frame_context
                    .camera
                    .update_from_screen_size(size.width as f32, size.height as f32);
                let _ = state.resize_window((size.width, size.height), &self.frame_context);
            }
            WindowEvent::RedrawRequested => {
                let scale_factor = self.window.as_ref().unwrap().scale_factor() as f32;
                self.egui_context
                    .as_mut()
                    .unwrap()
                    .set_pixels_per_point(scale_factor);
                let raw_input = self
                    .egui_winit
                    .as_mut()
                    .unwrap()
                    .take_egui_input(&self.window.as_ref().unwrap());
                let egui_stuff = self.egui_context.as_mut().unwrap().run(raw_input, |ctx| {
                    egui::Window::new("Zurie Engine UI").show(ctx, |ui| {
                        ui.label("Vulkan rendering with egui!");
                    });
                });

                self.egui_winit.as_mut().unwrap().handle_platform_output(
                    &self.window.as_ref().unwrap(),
                    egui_stuff.platform_output.clone(),
                );
                self.frame_context.egui_stuff = egui_stuff;
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
                    .render(&self.frame_context, objects.into_iter())
                    .unwrap();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}
