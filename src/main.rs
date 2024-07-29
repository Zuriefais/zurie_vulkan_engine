use log::info;
use winit::{application::ApplicationHandler, event_loop::EventLoop};

fn main() {
    env_logger::init();

    info!("Creating event loop");

    let event_loop = EventLoop::new().unwrap();
    let mut app = App();

    event_loop.run_app(&mut app).unwrap();
}

struct App();

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
    }
}
