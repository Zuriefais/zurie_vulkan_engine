use anyhow::Ok;
use egui::{self, Context};
use hashbrown::HashSet;
use log::{info, warn};
use std::sync::{Arc, RwLock};
use wasmtime::Engine;
use winit::event::{MouseScrollDelta, WindowEvent};
use zurie_shared::slotmap::{DefaultKey, SlotMap};
use zurie_types::{camera::Camera, glam::Vec2, KeyCode, Object};

use super::engine_mod::EngineMod;

pub struct ModManager {
    engine: Engine,
    gui_context: Context,
    mods: Vec<Arc<RwLock<EngineMod>>>,
    new_mod_path: String,
    pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
    mouse_pos: Arc<RwLock<Vec2>>,
    object_storage: Arc<RwLock<SlotMap<DefaultKey, Object>>>,
    camera: Arc<RwLock<Camera>>,
}

impl ModManager {
    pub fn event(&mut self, ev: WindowEvent) -> anyhow::Result<()> {
        if let WindowEvent::KeyboardInput { event, .. } = ev.clone() {
            match event.physical_key {
                winit::keyboard::PhysicalKey::Code(key_code) => {
                    let key_code = key_code as u32;
                    let key_code: KeyCode = KeyCode::try_from(key_code).unwrap();
                    for engine_mod in self.mods.iter() {
                        let mut mod_lock = engine_mod.write().unwrap();
                        mod_lock.key_event(key_code)?;
                    }
                }
                winit::keyboard::PhysicalKey::Unidentified(_) => {}
            }
        }
        if let WindowEvent::MouseWheel { delta, .. } = ev {
            if let MouseScrollDelta::LineDelta(_, y) = delta {
                for engine_mod in self.mods.iter() {
                    let mut mod_lock = engine_mod.write().unwrap();
                    mod_lock.scroll(y)?;
                }
            }
        }
        Ok(())
    }
    pub fn update(&mut self) -> anyhow::Result<()> {
        let mut reload_mods = false;
        let mut load_new_mod = false;
        egui::Window::new("Mods Window").show(&self.gui_context, |ui| {
            reload_mods = ui.button("reload mods").clicked();
            load_new_mod = ui.button("Load new mod").clicked();
            ui.label("mod path:");
            ui.text_edit_singleline(&mut self.new_mod_path);
        });
        if reload_mods {
            let mut new_mods = vec![];
            for engine_mod in self.mods.iter() {
                let mod_lock = engine_mod.read().unwrap();
                let mod_path = mod_lock.path.clone();
                new_mods.push(Arc::new(RwLock::new(EngineMod::new(
                    mod_path.clone(),
                    &self.engine,
                    self.gui_context.clone(),
                    self.pressed_keys_buffer.clone(),
                    self.mouse_pos.clone(),
                    self.object_storage.clone(),
                    self.camera.clone(),
                )?)));
                info!("reloading {}", mod_path);
            }
            self.mods = new_mods;
        }
        if load_new_mod {
            info!("Loading mod at path: {}", self.new_mod_path.clone());
            self.mods.push(Arc::new(RwLock::new(EngineMod::new(
                self.new_mod_path.clone(),
                &self.engine,
                self.gui_context.clone(),
                self.pressed_keys_buffer.clone(),
                self.mouse_pos.clone(),
                self.object_storage.clone(),
                self.camera.clone(),
            )?)));
        }
        for engine_mod in self.mods.iter() {
            let mut mod_lock = engine_mod.write().unwrap();
            if let Err(e) = mod_lock.update() {
                warn!("Error updating mod {}: {}", mod_lock.path, e);
                continue; // Skip this mod but continue with others
            }
        }
        Ok(())
    }
    pub fn new(
        gui_context: Context,
        pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
        mouse_pos: Arc<RwLock<Vec2>>,
        object_storage: Arc<RwLock<SlotMap<DefaultKey, Object>>>,
        camera: Arc<RwLock<Camera>>,
    ) -> Self {
        let engine = Engine::default();
        let test_mod = Arc::new(RwLock::new(
            EngineMod::new(
                "./target/wasm32-unknown-unknown/release/example_mod.wasm".to_string(),
                &engine,
                gui_context.clone(),
                pressed_keys_buffer.clone(),
                mouse_pos.clone(),
                object_storage.clone(),
                camera.clone(),
            )
            .expect("Error loading mod"),
        ));
        let mods = vec![test_mod];
        Self {
            engine,
            gui_context,
            mods,
            new_mod_path: String::new(),
            pressed_keys_buffer,
            mouse_pos,
            object_storage,
            camera,
        }
    }
}
