use std::sync::Arc;
use std::sync::RwLock;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;
use zurie_types::Object;
use zurie_types::camera::Camera;

pub trait RenderBackend: Sized {
    fn init(window: Arc<Window>, event_loop: &ActiveEventLoop) -> Result<Self, anyhow::Error>;
    fn render(
        &mut self,
        background_color: [f32; 4],
        camera: &Camera,
        objects: Arc<RwLock<Vec<Object>>>,
    ) -> anyhow::Result<()>;
    fn handle_window_event(&mut self, event: &winit::event::WindowEvent) -> anyhow::Result<()>;
    fn get_egui_context(&self) -> egui::Context;
    fn resize_window(&mut self, size: (u32, u32)) -> anyhow::Result<()>;
}
