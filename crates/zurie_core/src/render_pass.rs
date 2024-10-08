use crate::{
    pixels_draw::{self, PixelsDrawPipeline},
    render::Renderer,
};
use std::sync::Arc;
use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        RenderPassBeginInfo, SubpassBeginInfo, SubpassContents,
    },
    device::Queue,
    image::view::ImageView,
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    sync::GpuFuture,
};

/// A render pass which places an incoming image over the frame, filling it.
pub struct RenderPassPlaceOverFrame {
    gfx_queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    pixels_draw_pipeline: PixelsDrawPipeline,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
}

impl RenderPassPlaceOverFrame {
    pub fn new(app: &Renderer) -> RenderPassPlaceOverFrame {
        let render_pass = vulkano::single_pass_renderpass!(
            app.gfx_queue.device().clone(),
            attachments: {
                color: {
                    format: app.output_format,
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {},
            },
        )
        .unwrap();
        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let pixels_draw_pipeline = PixelsDrawPipeline::new(app, subpass);
        let gfx_queue = app.gfx_queue();

        RenderPassPlaceOverFrame {
            gfx_queue,
            render_pass,
            pixels_draw_pipeline,
            command_buffer_allocator: app.command_buffer_allocator.clone(),
        }
    }

    pub fn render<F>(
        &self,
        before_future: F,
        image_view: Arc<ImageView>,
        target: Arc<ImageView>,
        background_color: [f32; 4],
        camera: pixels_draw::vs::Camera,
    ) -> Box<dyn GpuFuture>
    where
        F: GpuFuture + 'static,
    {
        // Get the dimensions.
        let img_dims: [u32; 2] = target.image().extent()[0..2].try_into().unwrap();
        let framebuffer = Framebuffer::new(
            self.render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![target],
                ..Default::default()
            },
        )
        .unwrap();
        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.as_ref(),
            self.gfx_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        command_buffer_builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some(background_color.into())],
                    ..RenderPassBeginInfo::framebuffer(framebuffer)
                },
                SubpassBeginInfo {
                    contents: SubpassContents::SecondaryCommandBuffers,
                    ..Default::default()
                },
            )
            .unwrap();
        let cb = self
            .pixels_draw_pipeline
            .draw(img_dims, image_view, background_color, camera);

        command_buffer_builder.execute_commands(cb).unwrap();
        command_buffer_builder
            .end_render_pass(Default::default())
            .unwrap();

        // Build the command buffer.
        let command_buffer = command_buffer_builder.build().unwrap();
        let after_future = before_future
            .then_execute(self.gfx_queue.clone(), command_buffer)
            .unwrap();
        after_future.boxed()
    }
}
