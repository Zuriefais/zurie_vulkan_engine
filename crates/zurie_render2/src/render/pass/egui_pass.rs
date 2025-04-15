use super::super::backend::Backend;

use crate::resources::resource_manager::ResourceManager;
use ash::vk;
use egui::Context;
use egui_ash_renderer::Renderer;

use zurie_render_glue::FrameContext;

pub struct EGUIPass {
    egui_ctx: Context,
    egui_renderer: Renderer,
    textures_to_free: Option<Vec<egui::TextureId>>,
}

impl EGUIPass {
    pub fn new(backend: &Backend, egui_ctx: Context) -> anyhow::Result<Self> {
        let egui_renderer = Renderer::with_default_allocator(
            &backend.instance,
            backend.physical_device,
            backend.device.clone(),
            egui_ash_renderer::DynamicRendering {
                color_attachment_format: backend.swapchain_format,
                depth_attachment_format: None,
            },
            egui_ash_renderer::Options {
                srgb_framebuffer: true,
                ..Default::default()
            },
        )?;

        Ok(Self {
            egui_ctx,
            egui_renderer,
            textures_to_free: None,
        })
    }

    pub fn init(
        &mut self,
        _backend: &Backend,
        _swapchain_extent: vk::Extent2D,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn record(
        &mut self,
        command_buffer: vk::CommandBuffer,
        backend: &Backend,
        _resources: &ResourceManager,
        frame_context: &FrameContext,
    ) -> anyhow::Result<()> {
        if !frame_context.egui_stuff.textures_delta.free.is_empty() {
            self.textures_to_free = Some(frame_context.egui_stuff.textures_delta.free.clone());
        }

        if !frame_context.egui_stuff.textures_delta.set.is_empty() {
            self.egui_renderer.set_textures(
                backend.graphics_queue,
                backend.command_pool,
                frame_context.egui_stuff.textures_delta.set.as_slice(),
            )?;
        }

        let clipped_primitives = self.egui_ctx.tessellate(
            frame_context.egui_stuff.shapes.clone(),
            frame_context.egui_stuff.pixels_per_point,
        );
        self.egui_renderer.cmd_draw(
            command_buffer,
            backend.swapchain_extent,
            frame_context.egui_stuff.pixels_per_point,
            &clipped_primitives,
        )?;

        Ok(())
    }

    pub fn resize(&mut self, _backend: &Backend, _new_extent: vk::Extent2D) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn destroy(&mut self, _backend: &Backend) {
        // egui_renderer cleanup handled by Drop
    }
}
