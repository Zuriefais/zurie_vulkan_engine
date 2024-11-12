use log::info;
use winit::event_loop::EventLoop;
use zurie_core::app::App;

fn main() {
    env_logger::init();

    info!("Creating event loop");

    let event_loop = EventLoop::new().unwrap();

    let mut app = App::default();

    event_loop.run_app(&mut app).unwrap();
}
