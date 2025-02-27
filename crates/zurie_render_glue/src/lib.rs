use std::sync::Arc;
use std::sync::RwLock;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;
use zurie_types::Object;
use zurie_types::camera::Camera;

pub struct RenderConfig<'a> {
    pub window: Arc<Window>,
    pub event_loop: &'a ActiveEventLoop,
    pub egui_context: egui::Context,
}

pub struct FrameContext {
    pub background_color: [f32; 4],
    pub camera: Camera,
}

impl Default for FrameContext {
    fn default() -> Self {
        Self {
            background_color: [131.0 / 255.0, 165.0 / 255.0, 152.0 / 255.0, 1.0],
            camera: Camera::default(),
        }
    }
}

pub trait RenderBackend: Sized {
    fn init(config: RenderConfig) -> Result<Self, anyhow::Error>;

    fn render<I>(&mut self, frame_context: FrameContext, objects: I) -> anyhow::Result<()>
    where
        I: Iterator<Item = Object>;

    fn handle_window_event(&mut self, event: &winit::event::WindowEvent) -> anyhow::Result<()>;

    fn resize_window(&mut self, size: (u32, u32)) -> anyhow::Result<()>;
}
