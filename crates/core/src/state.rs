use std::sync::Arc;

use anyhow::Ok;
use camera::Camera;
use ecolor::hex_color;
use egui_winit_vulkano::egui::Context;
use glam::Vec2;
use input::InputState;
use log::info;
use std::sync::RwLock;
use wasmtime::Engine;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};

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
    input: InputState,
    selected_cell_type: CellType,
    background_color: [f32; 4],
    camera: Camera,
    mods: Vec<Arc<RwLock<EngineMod>>>,
    engine: Engine,
    gui_context: Context,
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
        let gui_context = gui.gui.context();
        let sim_clock = SimClock::default();
        let size = renderer.window_size();
        let mut camera = Camera::create_camera_from_screen_size(
            size[0] as f32,
            size[1] as f32,
            0.1,
            100.0,
            1.0,
            Vec2::ZERO,
        );
        camera.update_matrix();

        let engine = Engine::default();
        let test_mod = Arc::new(RwLock::new(
            EngineMod::new(
                "./target/wasm32-unknown-unknown/release/example_mod.wasm".to_string(),
                &engine,
                gui_context.clone(),
            )
            .expect("Error loading mod"),
        ));
        let mods = vec![test_mod];

        State {
            renderer,
            render_pipeline,
            gui,
            sim_clock,
            input: InputState::default(),
            selected_cell_type: CellType::Sand,
            background_color: hex_color!("#8FA3B3").to_normalized_gamma_f32(),
            camera,
            mods,
            engine,
            gui_context,
        }
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.sim_clock.clock();
        let mut reload_mods = false;
        self.gui.draw_gui(
            &mut self.sim_clock,
            &mut self.render_pipeline.compute,
            &mut self.input.mouse.hover_gui,
            &mut self.selected_cell_type,
            self.renderer.window_size(),
            &mut self.background_color,
            &mut reload_mods,
        );
        if reload_mods {
            let mut new_mods = vec![];
            for engine_mod in self.mods.iter() {
                let mod_lock = engine_mod.read().unwrap();
                let mod_path = mod_lock.path.clone();
                new_mods.push(Arc::new(RwLock::new(EngineMod::new(
                    mod_path.clone(),
                    &self.engine,
                    self.gui_context.clone(),
                )?)));
                info!("reloading {}", mod_path);
            }
            self.mods = new_mods;
        }
        for engine_mod in self.mods.iter() {
            let mut mod_lock = engine_mod.write().unwrap();
            mod_lock.update()?;
        }

        if self.input.mouse.left_pressed && !self.input.mouse.hover_gui {
            self.render_pipeline.compute.draw(
                self.input.mouse.position,
                self.renderer.window_size(),
                self.selected_cell_type,
            );
        }
        if self.input.mouse.right_pressed && !self.input.mouse.hover_gui {
            self.render_pipeline.compute.draw(
                self.input.mouse.position,
                self.renderer.window_size(),
                CellType::Empty,
            );
        }
        let before_pipeline_future = self.renderer.acquire()?;

        // Compute.
        let after_compute = self
            .render_pipeline
            .compute
            .compute(before_pipeline_future, self.sim_clock.simulate());

        // Render.
        let color_image = self.render_pipeline.compute.color_image();
        let target_image = self.renderer.swapchain_image_view();

        let after_render = self.render_pipeline.place_over_frame.render(
            after_compute,
            color_image,
            target_image.clone(),
            self.background_color,
            self.camera.uniform,
        );
        let after_gui = self.gui.draw_on_image(after_render, target_image);

        // Finish the frame. Wait for the future so resources are not in use when we render.
        self.renderer.present(after_gui, true);
        anyhow::Ok(())
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        self.renderer.resize();
        self.render_pipeline.compute.resize(size)
    }

    pub fn event(&mut self, ev: WindowEvent) {
        self.gui.event(&ev);
        self.input.event(ev.clone());
        self.camera.event(ev);
    }
}

use crate::{
    compute_sand::{CellType, SandComputePipeline},
    gui::GameGui,
    render::Renderer,
    render_pass::RenderPassPlaceOverFrame,
    scripting::EngineMod,
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
            sim_rate: 0,
            cur_sim: 0,
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

    pub fn ui_togles(&mut self) -> (&mut bool, &mut u16, &mut u16) {
        (
            &mut self.simulate_ui_togle,
            &mut self.cur_sim,
            &mut self.sim_rate,
        )
    }

    fn simulate(&mut self) -> bool {
        self.simulate
    }
}

pub mod camera;
pub mod input;
