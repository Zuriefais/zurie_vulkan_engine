use std::path::Path;
use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;
use winit::window::Window;
use zurie_types::camera::Camera;
use zurie_types::{Object, SpriteHandle};

pub struct RenderConfig<'a> {
    pub window: Arc<Window>,
    pub event_loop: &'a ActiveEventLoop,
    pub egui_context: egui::Context,
}

#[derive(Clone)]
pub struct FrameContext {
    pub background_color: [f32; 4],
    pub camera: Camera,
    pub egui_stuff: egui::FullOutput,
}

impl Default for FrameContext {
    fn default() -> Self {
        Self {
            background_color: [131.0 / 255.0, 165.0 / 255.0, 152.0 / 255.0, 1.0],
            camera: Camera::default(),
            egui_stuff: Default::default(),
        }
    }
}

pub trait RenderBackend: Sized {
    fn init(config: RenderConfig) -> Result<Self, anyhow::Error>;

    fn render<I>(&mut self, frame_context: &FrameContext, objects: I) -> anyhow::Result<()>
    where
        I: Iterator<Item = Object>;

    fn handle_window_event(&mut self, event: &winit::event::WindowEvent) -> anyhow::Result<()>;

    fn resize_window(
        &mut self,
        size: (u32, u32),
        frame_context: &FrameContext,
    ) -> anyhow::Result<()>;

    fn get_sprite_manager(&self) -> Box<dyn SpriteManager>;
}

pub enum LoadSpriteInfo {
    Path(Box<Path>),
    Buffer(Vec<u8>),
}

pub trait SpriteManager {
    fn load_sprite(&self, info: LoadSpriteInfo) -> SpriteHandle;
}
