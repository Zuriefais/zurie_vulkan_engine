use log::info;
use std::{sync::Arc, time::Instant};
use tracy_client::{Client, set_thread_name};

use zurie_shared::DELTA_TIME;

use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::state::State;

pub struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    delta_time: Instant,
    tracy_client: Client,
}

impl Default for App {
    fn default() -> Self {
        let tracy_client = Client::start();
        set_thread_name!("Main Thread");
        Self {
            delta_time: Instant::now(),
            window: Default::default(),
            state: None,
            tracy_client,
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

            let state = pollster::block_on(State::new(window.clone(), event_loop));
            self.state = Some(state);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        self.state.as_mut().unwrap().event(event.clone()).unwrap();
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
            WindowEvent::Resized(size) => self
                .state
                .as_mut()
                .unwrap()
                .resize([size.width, size.height]),
            WindowEvent::RedrawRequested => {
                self.state.as_mut().unwrap().render().unwrap();
                unsafe {
                    DELTA_TIME = self.delta_time.elapsed().as_secs_f32();
                }
                self.delta_time = Instant::now();
                self.window.as_ref().unwrap().request_redraw();
                self.tracy_client.frame_mark();
            }
            _ => {}
        }
    }
}
