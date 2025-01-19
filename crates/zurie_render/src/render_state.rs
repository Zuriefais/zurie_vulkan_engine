use std::sync::{Arc, RwLock};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};
use zurie_types::{camera::Camera, glam::Vec2, Object};

use crate::{
    compute_sand::{CellType, SandComputePipeline},
    gui::GuiRender,
    object_draw::render_pass::ObjectRenderPass,
    pixels_draw::render_pass::PixelsRenderPass,
    render::Renderer,
    sprite::SpriteManager,
};

pub struct RenderState {
    pub compute: SandComputePipeline,
    pub pixels_render: PixelsRenderPass,
    pub objects_render: ObjectRenderPass,
    pub renderer: Renderer,
    pub gui: GuiRender,
    pub sprite_manager: Arc<RwLock<SpriteManager>>,
}

impl RenderState {
    pub fn new(window: Arc<Window>, event_loop: &ActiveEventLoop) -> anyhow::Result<RenderState> {
        let renderer = Renderer::new(window);
        let gui = GuiRender::new(
            event_loop,
            renderer.surface(),
            renderer.gfx_queue.clone(),
            renderer.output_format,
        );

        let sprite_manager: Arc<RwLock<SpriteManager>> =
            Arc::new(RwLock::new(SpriteManager::new(gui.gui.context())));

        Ok(RenderState {
            compute: SandComputePipeline::new(&renderer),
            pixels_render: PixelsRenderPass::new(&renderer),
            objects_render: ObjectRenderPass::new(&renderer, sprite_manager.clone())?,
            renderer,
            gui,
            sprite_manager,
        })
    }

    pub fn render(
        &mut self,
        background_color: [f32; 4],
        camera: &Camera,
        objects: Arc<RwLock<Vec<Object>>>,
    ) -> anyhow::Result<()> {
        {
            let mut sprite_manager = self.sprite_manager.write().unwrap();
            sprite_manager.process_queue(
                self.renderer.memory_allocator.clone(),
                self.renderer.command_buffer_allocator.clone(),
                self.renderer.gfx_queue.clone(),
            )?;
            sprite_manager.gui();
        }

        let before_pipeline_future = self.renderer.acquire()?;

        // Compute.
        // let after_compute = self
        //     .compute
        //     .compute(before_pipeline_future, sim_clock.simulate());

        // Render.
        //let color_image = self.compute.color_image();
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
            before_pipeline_future,
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
