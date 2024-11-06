pub mod gui;
pub mod input;

use ecolor::hex_color;
use input::InputState;
use log::info;
use std::sync::{Arc, RwLock};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};
use zurie_ecs::{Architype, ComponentID, World};
use zurie_render::{compute_sand::CellType, render_state::RenderState};
use zurie_scripting::mod_manager::ModManager;
use zurie_shared::sim_clock::SimClock;
use zurie_types::serde::Deserialize;
use zurie_types::{camera::Camera, flexbuffers, glam::Vec2, Object, Vector2};

pub struct State {
    //gui: GameGui,
    pub sim_clock: SimClock,
    input: InputState,
    selected_cell_type: CellType,
    background_color: [f32; 4],
    camera: Arc<RwLock<Camera>>,
    mod_manager: ModManager,
    world: Arc<RwLock<World>>,
    render_state: RenderState,
    pos_component: ComponentID,
    scale_component: ComponentID,
    color_component: ComponentID,
}

impl State {
    pub async fn new(window: Arc<Window>, event_loop: &ActiveEventLoop) -> State {
        let render_state = RenderState::new(window, event_loop);
        let gui_context = render_state.gui.gui.context();
        //let gui = GameGui::new(gui_context.clone());

        let sim_clock = SimClock::default();
        let size = render_state.renderer.window_size();
        let camera = Arc::new(RwLock::new(Camera::create_camera_from_screen_size(
            size[0] as f32,
            size[1] as f32,
            0.1,
            100.0,
            1.0,
            Vec2::ZERO,
        )));
        let input = InputState::default();
        let (world, pos_component, scale_component, color_component) = {
            let mut world: World = Default::default();
            let pos_component = world.register_component("position".into());
            let scale_component = world.register_component("scale".into());
            let color_component = world.register_component("color".into());
            (
                Arc::new(RwLock::new(world)),
                pos_component,
                scale_component,
                color_component,
            )
        };
        let mod_manager = ModManager::new(
            gui_context.clone(),
            input.pressed_keys_buffer.clone(),
            input.mouse.position.clone(),
            world.clone(),
            camera.clone(),
        );

        State {
            //gui,
            sim_clock,
            input,
            selected_cell_type: CellType::Sand,
            background_color: hex_color!("#8FA3B3").to_normalized_gamma_f32(),
            camera,
            mod_manager,
            world,
            render_state,
            pos_component,
            scale_component,
            color_component,
        }
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.sim_clock.clock();
        self.render_state.gui.start_gui();
        // self.gui.draw_gui(
        //     &mut self.sim_clock,
        //     &mut self.render_state.compute,
        //     &mut self.input.mouse.hover_gui,
        //     &mut self.selected_cell_type,
        //     self.render_state.renderer.window_size(),
        //     &mut self.background_color,
        // );
        self.mod_manager.update()?;
        let mut objects: Vec<Object> = self
            .world
            .read()
            .unwrap()
            .get_entities_with_arhetype(Architype {
                data: vec![
                    self.pos_component,
                    self.scale_component,
                    self.color_component,
                ],
            })
            .iter()
            .map(|(_, entity_data)| {
                let mut obj = Object::default();
                for (component_id, component_data) in entity_data.data.iter() {
                    if *component_id == self.pos_component {
                        obj.position = match component_data {
                            zurie_ecs::ComponentData::Vector(vector2) => *vector2,
                            _ => Vec2::ZERO.into(),
                        };
                    } else if *component_id == self.scale_component {
                        obj.scale = match component_data {
                            zurie_ecs::ComponentData::Scale(scale) => *scale,
                            _ => [1.0, 1.0],
                        };
                    } else if *component_id == self.color_component {
                        obj.color = match component_data {
                            zurie_ecs::ComponentData::Color(color) => *color,
                            _ => [1.0, 1.0, 1.0, 1.0],
                        };
                    }
                }
                obj
            })
            .collect();
        objects.push(Object::default());
        info!("objects count, {}", objects.len());
        let objects = Arc::new(RwLock::new(objects));

        self.render_state.render(
            &mut self.sim_clock,
            self.selected_cell_type,
            &self.input.mouse.position.read().unwrap(),
            self.input.mouse.left_pressed,
            self.input.mouse.right_pressed,
            self.input.mouse.hover_gui,
            self.background_color,
            &self.camera.read().unwrap(),
            objects,
        )?;
        self.input.after_update();

        anyhow::Ok(())
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        self.render_state.resize(size);
        self.camera
            .write()
            .unwrap()
            .update_matrix_from_screen_size(size[0] as f32, size[1] as f32);
    }

    pub fn event(&mut self, ev: WindowEvent) -> anyhow::Result<()> {
        self.render_state.event(&ev)?;
        self.input.event(ev.clone());
        // if let WindowEvent::MouseWheel { delta, .. } = ev {
        //     if let MouseScrollDelta::LineDelta(_, y) = delta {
        //         self.camera.write().unwrap().event(y);
        //     }
        // }

        self.mod_manager.window_event(ev)?;
        Ok(())
    }
}
