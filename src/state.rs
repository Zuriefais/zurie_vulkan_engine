use std::sync::Arc;

use egui_winit_vulkano::{egui, Gui, GuiConfig};
use log::info;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};

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
    gui: Gui,
    simulate: bool,
}

impl State {
    pub async fn new(window: Arc<Window>, event_loop: &ActiveEventLoop) -> State {
        let renderer = Renderer::new(window);
        let render_pipeline = RenderPipeline::new(&renderer);
        let gui = Gui::new(
            event_loop,
            renderer.surface(),
            renderer.gfx_queue(),
            renderer.output_format,
            GuiConfig {
                allow_srgb_render_target: true,
                is_overlay: true,
                samples: vulkano::image::SampleCount::Sample1,
            },
        );
        // let shader = wgsl_to_shader_module(
        //     "test.wgsl".to_string(),
        //     renderer.device.clone(),
        //     "main".to_string(),
        //     naga::ShaderStage::Compute,
        // );
        State {
            renderer,
            render_pipeline,
            gui,
            simulate: true,
        }
    }

    pub fn render(&mut self) {
        self.gui.immediate_ui(|gui| {
            let ctx = gui.context();
            egui::Window::new("Debug window").show(&ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(egui::widgets::Label::new("Hi there!"));
                    if ui.button("Click me else you die").clicked() {
                        info!("it's joke")
                    }
                    ui.checkbox(&mut self.simulate, "Simulate")
                });
            });
        });
        let before_pipeline_future = match self.renderer.acquire() {
            Err(e) => {
                println!("{e}");
                return;
            }
            Ok(future) => future,
        };

        // Compute.
        let after_compute = self
            .render_pipeline
            .compute
            .compute(before_pipeline_future, &self.simulate);

        // Render.
        let color_image = self.render_pipeline.compute.color_image();
        let target_image = self.renderer.swapchain_image_view();

        let after_render = self.render_pipeline.place_over_frame.render(
            after_compute,
            color_image,
            target_image.clone(),
        );
        let after_gui = self.gui.draw_on_image(after_render, target_image);

        // Finish the frame. Wait for the future so resources are not in use when we render.
        self.renderer.present(after_gui, true);
    }

    pub fn resize(&mut self) {
        self.renderer.resize();
    }

    pub fn event(&mut self, ev: WindowEvent) {
        self.gui.update(&ev);
    }
}

use crate::{
    compute_render::RenderComputePipeline, render::Renderer, render_pass::RenderPassPlaceOverFrame,
};
