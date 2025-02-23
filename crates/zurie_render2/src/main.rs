use winit::event_loop::EventLoop;
use zurie_render2::render_state::App;

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::default();
    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");
}
