use std::sync::Arc;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};
use zurie_shared::sim_clock::SimClock;
use zurie_types::{camera::Camera, glam::Vec2, Object};

use crate::{
    compute_sand::{CellType, SandComputePipeline},
    gui::GuiRender,
    object_draw::render_pass::ObjectRenderPass,
    pixels_draw::render_pass::PixelsRenderPass,
    render::Renderer,
};

pub struct RenderState {
    pub compute: SandComputePipeline,
    pub pixels_render: PixelsRenderPass,
    pub objects_render: ObjectRenderPass,
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
            renderer.output_format,
        );
        RenderState {
            compute: SandComputePipeline::new(&renderer),
            pixels_render: PixelsRenderPass::new(&renderer),
            objects_render: ObjectRenderPass::new(&renderer),
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
        camera: &Camera,
        objects: &[Object],
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

        // let after_pixels_render = self.pixels_render.render(
        //     after_compute,
        //     color_image,
        //     target_image.clone(),
        //     background_color,
        //     camera,
        //     //&self.object_storage.read().unwrap(),
        // );

        let after_objects_render = self.objects_render.render(
            after_compute,
            target_image.clone(),
            background_color,
            camera,
            objects,
        );
        let after_gui = self.gui.draw_on_image(after_objects_render, target_image);

        // Finish the frame. Wait for the future so resources are not in use when we render.
        self.renderer.present(after_gui, true);

        Ok(())
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        self.renderer.resize();
        self.compute.resize(size)
    }
    pub fn event(&mut self, ev: &WindowEvent) -> anyhow::Result<()> {
        self.gui.event(ev);
        Ok(())
    }
}
