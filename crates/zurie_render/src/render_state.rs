use std::sync::Arc;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};
use zurie_shared::{camera::Camera, sim_clock::SimClock};
use zurie_types::glam::Vec2;

use crate::{
    compute_sand::{CellType, SandComputePipeline},
    gui::GuiRender,
    pixels_draw::render_pass::PixelsRenderPass,
    render::Renderer,
};

pub struct RenderState {
    pub compute: SandComputePipeline,
    pub place_over_frame: PixelsRenderPass,
    pub renderer: Renderer,
    pub gui: GuiRender,
}

impl RenderState {
    pub fn new(window: Arc<Window>, event_loop: &ActiveEventLoop) -> RenderState {
        let renderer = Renderer::new(window);
        let gui = GuiRender::new(
            event_loop,
            renderer.surface(),
            renderer.gfx_queue.clone(),
            renderer.output_format.clone(),
        );
        RenderState {
            compute: SandComputePipeline::new(&renderer),
            place_over_frame: PixelsRenderPass::new(&renderer),
            renderer,
            gui,
        }
    }

    pub fn render(
        &mut self,
        sim_clock: &mut SimClock,
        selected_cell_type: CellType,
        mouse_pos: &Vec2,
        left_pressed: bool,
        right_pressed: bool,
        hover_gui: bool,
        background_color: [f32; 4],
        camera: Camera,
    ) -> anyhow::Result<()> {
        if left_pressed && !hover_gui {
            self.compute
                .draw(*mouse_pos, self.renderer.window_size(), selected_cell_type);
        }
        if right_pressed && !hover_gui {
            self.compute
                .draw(*mouse_pos, self.renderer.window_size(), CellType::Empty);
        }
        let before_pipeline_future = self.renderer.acquire()?;

        // Compute.
        let after_compute = self
            .compute
            .compute(before_pipeline_future, sim_clock.simulate());

        // Render.
        let color_image = self.compute.color_image();
        let target_image = self.renderer.swapchain_image_view();

        let after_render = self.place_over_frame.render(
            after_compute,
            color_image,
            target_image.clone(),
            background_color,
            camera,
            //&self.object_storage.read().unwrap(),
        );
        let after_gui = self.gui.draw_on_image(after_render, target_image);

        // Finish the frame. Wait for the future so resources are not in use when we render.
        self.renderer.present(after_gui, true);

        Ok(())
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        self.renderer.resize();
        self.compute.resize(size)
    }
    pub fn event(&mut self, ev: &WindowEvent) -> anyhow::Result<()> {
        self.gui.event(&ev);
        Ok(())
    }
}