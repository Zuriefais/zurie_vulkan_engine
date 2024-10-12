pub mod input;

use ecolor::hex_color;
use input::InputState;
use std::sync::Arc;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};
use zurie_render::{
    compute_sand::CellType, gui::GameGui, render::Renderer, render_state::RenderPipeline,
};
use zurie_scripting::mod_manager::ModManager;
use zurie_shared::{camera::Camera, sim_clock::SimClock};
use zurie_types::glam::Vec2;

pub struct State {
    pub render_pipeline: RenderPipeline,
    renderer: Renderer,
    gui: GameGui,
    pub sim_clock: SimClock,
    input: InputState,
    selected_cell_type: CellType,
    background_color: [f32; 4],
    camera: Camera,
    mod_manager: ModManager,
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
        let camera = Camera::create_camera_from_screen_size(
            size[0] as f32,
            size[1] as f32,
            0.1,
            100.0,
            1.0,
            Vec2::ZERO,
        );
        let input = InputState::default();
        let object_storage = Default::default();
        let mod_manager = ModManager::new(
            gui_context.clone(),
            input.pressed_keys_buffer.clone(),
            input.mouse.position.clone(),
            object_storage,
        );

        State {
            renderer,
            render_pipeline,
            gui,
            sim_clock,
            input,
            selected_cell_type: CellType::Sand,
            background_color: hex_color!("#8FA3B3").to_normalized_gamma_f32(),
            camera,
            mod_manager,
        }
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.sim_clock.clock();

        self.gui.draw_gui(
            &mut self.sim_clock,
            &mut self.render_pipeline.compute,
            &mut self.input.mouse.hover_gui,
            &mut self.selected_cell_type,
            self.renderer.window_size(),
            &mut self.background_color,
        );
        self.mod_manager.update()?;

        if self.input.mouse.left_pressed && !self.input.mouse.hover_gui {
            self.render_pipeline.compute.draw(
                *self.input.mouse.position.read().unwrap(),
                self.renderer.window_size(),
                self.selected_cell_type,
            );
        }
        if self.input.mouse.right_pressed && !self.input.mouse.hover_gui {
            self.render_pipeline.compute.draw(
                *self.input.mouse.position.read().unwrap(),
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
            self.camera,
        );
        let after_gui = self.gui.draw_on_image(after_render, target_image);

        // Finish the frame. Wait for the future so resources are not in use when we render.
        self.renderer.present(after_gui, true);
        self.input.after_update();
        anyhow::Ok(())
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        self.renderer.resize();
        self.render_pipeline.compute.resize(size)
    }

    pub fn event(&mut self, ev: WindowEvent) -> anyhow::Result<()> {
        self.gui.event(&ev);
        self.input.event(ev.clone());
        self.camera.event(ev.clone());
        self.mod_manager.event(ev)?;
        Ok(())
    }
}
