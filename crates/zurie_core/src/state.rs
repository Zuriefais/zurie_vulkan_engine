pub mod gui;
pub mod input;

use ecolor::hex_color;
use gui::GameGui;
use input::InputState;
use std::sync::{Arc, RwLock};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};
use zurie_render::{compute_sand::CellType, render_state::RenderState};
use zurie_scripting::mod_manager::ModManager;
use zurie_shared::{camera::Camera, sim_clock::SimClock};
use zurie_types::{glam::Vec2, Object};

pub struct State {
    gui: GameGui,
    pub sim_clock: SimClock,
    input: InputState,
    selected_cell_type: CellType,
    background_color: [f32; 4],
    camera: Camera,
    mod_manager: ModManager,
    object_storage: Arc<RwLock<Vec<Object>>>,
    render_state: RenderState,
}

impl State {
    pub async fn new(window: Arc<Window>, event_loop: &ActiveEventLoop) -> State {
        let render_state = RenderState::new(window, event_loop);
        let gui_context = render_state.gui.gui.context();
        let gui = GameGui::new(gui_context.clone());

        let sim_clock = SimClock::default();
        let size = render_state.renderer.window_size();
        let camera = Camera::create_camera_from_screen_size(
            size[0] as f32,
            size[1] as f32,
            0.1,
            100.0,
            1.0,
            Vec2::ZERO,
        );
        let input = InputState::default();
        let object_storage: Arc<RwLock<Vec<Object>>> = Default::default();
        let mod_manager = ModManager::new(
            gui_context.clone(),
            input.pressed_keys_buffer.clone(),
            input.mouse.position.clone(),
            object_storage.clone(),
        );

        State {
            gui,
            sim_clock,
            input,
            selected_cell_type: CellType::Sand,
            background_color: hex_color!("#8FA3B3").to_normalized_gamma_f32(),
            camera,
            mod_manager,
            object_storage,
            render_state,
        }
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.sim_clock.clock();
        self.render_state.gui.start_gui();
        self.gui.draw_gui(
            &mut self.sim_clock,
            &mut self.render_state.compute,
            &mut self.input.mouse.hover_gui,
            &mut self.selected_cell_type,
            self.render_state.renderer.window_size(),
            &mut self.background_color,
        );
        self.mod_manager.update()?;
        self.render_state.render(
            &mut self.sim_clock,
            self.selected_cell_type,
            &self.input.mouse.position.read().unwrap(),
            self.input.mouse.left_pressed,
            self.input.mouse.right_pressed,
            self.input.mouse.hover_gui,
            self.background_color,
            self.camera,
            &self.object_storage.read().unwrap(),
        )?;
        self.input.after_update();

        anyhow::Ok(())
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        self.render_state.resize(size);
    }

    pub fn event(&mut self, ev: WindowEvent) -> anyhow::Result<()> {
        self.render_state.event(&ev)?;
        self.input.event(ev.clone());
        self.camera.event(ev.clone());
        self.mod_manager.event(ev)?;
        Ok(())
    }
}
