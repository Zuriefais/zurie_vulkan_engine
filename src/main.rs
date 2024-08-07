pub mod app;
pub mod compute_render;
pub mod gui;
pub mod pixels_draw;
pub mod render;
pub mod render_pass;
pub mod state;

use app::App;
use log::info;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    info!("Creating event loop");

    let event_loop = EventLoop::new().unwrap();

    let mut app = App::default();

    event_loop.run_app(&mut app).unwrap();
}
