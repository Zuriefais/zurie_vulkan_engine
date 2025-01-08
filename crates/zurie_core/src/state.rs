pub mod gui;
pub mod input;

use ecolor::hex_color;
use egui_winit_vulkano::egui::Context;
use input::InputState;

use std::sync::{Arc, RwLock};
#[cfg(target_os = "android")]
use winit::platform::android::ActiveEventLoopExtAndroid;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};
use zurie_ecs::{Architype, ComponentID, World};
use zurie_render::{compute_sand::CellType, render_state::RenderState};
use zurie_scripting::mod_manager::ModManager;
use zurie_types::{camera::Camera, glam::Vec2, ComponentData, Object};

pub struct State {
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
    sprite_component: ComponentID,
    gui_context: Context,
}

impl State {
    pub async fn new(window: Arc<Window>, event_loop: &ActiveEventLoop) -> State {
        let render_state =
            RenderState::new(window, event_loop).expect("error creating render state");
        let gui_context = render_state.gui.gui.context();

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
        let (world, pos_component, scale_component, color_component, sprite_component) = {
            let mut world: World = Default::default();
            let pos_component = world.register_component("position".into());
            let scale_component = world.register_component("scale".into());
            let color_component = world.register_component("color".into());
            let sprite_component = world.register_component("sprite".into());
            (
                Arc::new(RwLock::new(world)),
                pos_component,
                scale_component,
                color_component,
                sprite_component,
            )
        };
        #[cfg(not(target_os = "android"))]
        let mod_manager = ModManager::new(
            gui_context.clone(),
            input.pressed_keys_buffer.clone(),
            input.mouse.position.clone(),
            world.clone(),
            camera.clone(),
            render_state.sprite_manager.clone(),
        );
        #[cfg(target_os = "android")]
        let mod_manager = ModManager::new(
            gui_context.clone(),
            input.pressed_keys_buffer.clone(),
            input.mouse.position.clone(),
            world.clone(),
            camera.clone(),
            render_state.sprite_manager.clone(),
            event_loop.android_app().clone(),
        );

        State {
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
            sprite_component,
            gui_context,
        }
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
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
        self.world
            .write()
            .unwrap()
            .inspector(self.gui_context.clone());
        let mut objects: Vec<Object> = self
            .world
            .read()
            .unwrap()
            .get_entities_data_with_arhetype(Architype {
                required: vec![self.pos_component],
                optional: vec![
                    self.scale_component,
                    self.color_component,
                    self.sprite_component,
                ],
            })
            .iter()
            .map(|(_, entity_data)| {
                let mut obj = Object::default();
                for (component_id, component_data) in entity_data.data.iter() {
                    if *component_id == self.pos_component {
                        obj.position = match component_data {
                            ComponentData::Vector(vector2) => *vector2,
                            _ => Vec2::ZERO.into(),
                        };
                    } else if *component_id == self.scale_component {
                        obj.scale = match component_data {
                            ComponentData::Vector(scale) => Into::<[f32; 2]>::into(*scale),
                            _ => [1.0, 1.0],
                        };
                    } else if *component_id == self.color_component {
                        obj.color = match component_data {
                            ComponentData::Color(color) => *color,
                            _ => [1.0, 1.0, 1.0, 1.0],
                        };
                    } else if *component_id == self.sprite_component {
                        obj.sprite = match component_data {
                            ComponentData::Sprite(handle) => *handle,
                            _ => 0,
                        };
                    }
                }
                obj
            })
            .collect();
        if objects.len() == 0 {
            objects.push(Object::default())
        };
        let objects = Arc::new(RwLock::new(objects));

        self.render_state.render(
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
