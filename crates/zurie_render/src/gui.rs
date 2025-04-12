use std::sync::Arc;

use egui_winit_vulkano::{Gui, GuiConfig};
use vulkano::{
    device::Queue, format::Format, image::view::ImageView, swapchain::Surface, sync::GpuFuture,
};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop};

pub struct GuiRender {
    pub gui: Gui,
}

impl GuiRender {
    pub fn new(
        event_loop: &ActiveEventLoop,
        surface: Arc<Surface>,
        gfx_queue: Arc<Queue>,
        output_format: Format,
    ) -> Self {
        let gui = Gui::new(
            event_loop,
            surface,
            gfx_queue,
            output_format,
            GuiConfig {
                allow_srgb_render_target: true,
                is_overlay: true,
                samples: vulkano::image::SampleCount::Sample1,
            },
        );

        GuiRender { gui }
    }

    pub fn event(&mut self, event: &WindowEvent) {
        self.gui.update(event);
    }

    pub fn start_gui(&mut self) {
        self.gui.immediate_ui(|_| {});
    }

    pub fn draw_on_image<F>(
        &mut self,
        before_future: F,
        final_image: Arc<ImageView>,
    ) -> Box<dyn GpuFuture>
    where
        F: GpuFuture + 'static,
    {
        self.gui.draw_on_image(before_future, final_image)
    }
}
