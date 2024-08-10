use std::sync::Arc;

use glam::Vec2;
use log::info;
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    window::Window,
};

pub struct RenderPipeline {
    pub compute: SandComputePipeline,
    pub place_over_frame: RenderPassPlaceOverFrame,
}

impl RenderPipeline {
    pub fn new(renderer: &Renderer) -> RenderPipeline {
        RenderPipeline {
            compute: SandComputePipeline::new(renderer),
            place_over_frame: RenderPassPlaceOverFrame::new(renderer),
        }
    }
}

pub struct State {
    pub render_pipeline: RenderPipeline,
    renderer: Renderer,
    gui: GameGui,
    pub sim_clock: SimClock,
    mouse: MouseState,
}

impl State {
    pub async fn new(window: Arc<Window>, event_loop: &ActiveEventLoop) -> State {
        let renderer = Renderer::new(window);
        let render_pipeline = RenderPipeline::new(&renderer);
        let gui = GameGui::new(
            event_loop,
            renderer.surface(),
            renderer.gfx_queue.clone(),
            renderer.output_format,
        );
        let sim_clock = SimClock::default();
        State {
            renderer,
            render_pipeline,
            gui,
            sim_clock,
            mouse: MouseState::default(),
        }
    }

    pub fn render(&mut self) {
        self.sim_clock.clock();
        self.gui.draw_gui(
            &mut self.sim_clock,
            &mut self.render_pipeline.compute,
            &mut self.mouse.hover_gui,
        );
        if self.mouse.pressed && !self.mouse.hover_gui {
            self.render_pipeline
                .compute
                .draw_grid(self.mouse.position.as_ivec2());
        }
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
            .compute(before_pipeline_future, &self.sim_clock.simulate());

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

    pub fn resize(&mut self, size: [u32; 2]) {
        self.renderer.resize();
        self.render_pipeline.compute.resize(size)
    }

    pub fn event(&mut self, ev: WindowEvent) {
        self.gui.event(&ev);
        self.mouse.event(ev);
    }
}

use crate::{
    compute_sand::SandComputePipeline, gui::GameGui, render::Renderer,
    render_pass::RenderPassPlaceOverFrame,
};

pub struct SimClock {
    simulate: bool,
    simulate_ui_togle: bool,
    sim_rate: u16,
    cur_sim: u16,
}

impl Default for SimClock {
    fn default() -> Self {
        SimClock {
            simulate: true,
            simulate_ui_togle: true,
            sim_rate: 1,
            cur_sim: 1,
        }
    }
}

impl SimClock {
    pub fn clock(&mut self) {
        if self.cur_sim == self.sim_rate {
            self.simulate = true;
            self.sim_rate = 0;
        } else if self.simulate_ui_togle {
            self.simulate = false;
            self.sim_rate += 1;
        }
        if !self.simulate_ui_togle {
            self.simulate = false;
        }
    }

    pub fn ui_togles(&mut self) -> (&mut bool, &mut u16, u16) {
        (
            &mut self.simulate_ui_togle,
            &mut self.cur_sim,
            self.sim_rate,
        )
    }

    fn simulate(&mut self) -> bool {
        self.simulate
    }
}

#[derive(Default)]
struct MouseState {
    position: Vec2,
    pressed: bool,
    hover_gui: bool,
}

impl MouseState {
    pub fn event(&mut self, ev: WindowEvent) {
        match ev {
            WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                (ElementState::Pressed, MouseButton::Left) => {
                    self.pressed = true;
                    info!("mouse pressed");
                }
                (ElementState::Released, MouseButton::Left) => {
                    self.pressed = false;
                    info!("mouse released");
                }
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.position = Vec2::new(position.x as f32, position.y as f32)
            }
            _ => {}
        }
    }
}
