use crate::{pixels_draw::PixelsDrawPipeline, render::Renderer};
use log::info;
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

    /// Places the view exactly over the target swapchain image. The texture draw pipeline uses a
    /// quad onto which it places the view.
    pub fn render<F>(
        &self,
        before_future: F,
        image_view: Arc<ImageView>,
        target: Arc<ImageView>,
    ) -> Box<dyn GpuFuture>
    where
        F: GpuFuture + 'static,
    {
        // Get the dimensions.
        let img_dims: [u32; 2] = target.image().extent()[0..2].try_into().unwrap();

        // Create the framebuffer.
        let framebuffer = Framebuffer::new(
            self.render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![target],
                ..Default::default()
            },
        )
        .unwrap();

        // Create a primary command buffer builder.
        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.as_ref(),
            self.gfx_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        // Begin the render pass.
        info!("before crash! 1");
        command_buffer_builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.0; 4].into())],
                    ..RenderPassBeginInfo::framebuffer(framebuffer)
                },
                SubpassBeginInfo {
                    contents: SubpassContents::SecondaryCommandBuffers,
                    ..Default::default()
                },
            )
            .unwrap();
        info!("before crash! 2");

        // Create a secondary command buffer from the texture pipeline & send draw commands.
        let cb = self.pixels_draw_pipeline.draw(img_dims, image_view);
        info!("before crash! 3");

        // Execute above commands (subpass).
        command_buffer_builder.execute_commands(cb).unwrap();
        info!("before crash! 3");

        // End the render pass.
        command_buffer_builder
            .end_render_pass(Default::default())
            .unwrap();
        info!("before crash! 4");

        // Build the command buffer.
        let command_buffer = command_buffer_builder.build().unwrap();
        info!("before crash! 5");
        // Execute primary command buffer.
        // let after_future = before_future
        //     .then_execute(self.gfx_queue.clone(), command_buffer)
        //     .unwrap();
        info!("before crash! 5");
        info!("after crash..");
        before_future.boxed()
    }
}
