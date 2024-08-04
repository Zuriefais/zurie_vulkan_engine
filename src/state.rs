use std::sync::Arc;

use ecolor::hex_color;

use winit::window::Window;

pub struct RenderPipeline {
    pub compute: RenderComputePipeline,
    pub place_over_frame: RenderPassPlaceOverFrame,
}

impl RenderPipeline {
    pub fn new(renderer: &Renderer) -> RenderPipeline {
        RenderPipeline {
            compute: RenderComputePipeline::new(renderer),
            place_over_frame: RenderPassPlaceOverFrame::new(renderer),
        }
    }
}

pub struct State {
    render_pipeline: RenderPipeline,
    renderer: Renderer,
}

impl State {
    pub async fn new(window: Arc<Window>) -> State {
        let renderer = Renderer::new(window);
        let render_pipeline = RenderPipeline::new(&renderer);
        State {
            renderer,
            render_pipeline,
        }
    }

    pub fn render(&mut self, window: Arc<Window>) {
        let before_pipeline_future = match self.renderer.acquire() {
            Err(e) => {
                println!("{e}");
                return;
            }
            Ok(future) => future,
        };

        // Compute.
        let after_compute =
            self.render_pipeline
                .compute
                .compute(before_pipeline_future, ALIVE_COLOR, DEAD_COLOR);

        // Render.
        let color_image = self.render_pipeline.compute.color_image();
        let target_image = self.renderer.swapchain_image_view();

        let after_render =
            self.render_pipeline
                .place_over_frame
                .render(after_compute, color_image, target_image);

        // Finish the frame. Wait for the future so resources are not in use when we render.
        self.renderer.present(after_render, true);
    }

    pub fn resize(&mut self) {
        self.renderer.resize();
    }
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450
            layout(location = 0) in vec3 position;

            void main() {
                gl_Position = vec4(position, 1.0);
            }
        "
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        "
    }
}

use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex};

use crate::{
    compute_render::RenderComputePipeline, render::Renderer, render_pass::RenderPassPlaceOverFrame,
};

#[derive(BufferContents, Vertex)]
#[repr(C)]
struct MyVertex {
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],
}

static DEAD_COLOR: [u8; 3] = color_hex::color_from_hex!("#0D2132");
static ALIVE_COLOR: [u8; 3] = color_hex::color_from_hex!("#D7572A");
