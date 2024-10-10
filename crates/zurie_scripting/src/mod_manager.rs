use anyhow::Ok;
use egui_winit_vulkano::egui::{self, Context};
use hashbrown::HashSet;
use log::info;
use std::sync::{Arc, RwLock};
use wasmtime::Engine;
use winit::event::WindowEvent;
use zurie_types::KeyCode;

use super::engine_mod::EngineMod;

pub struct ModManager {
    engine: Engine,
    gui_context: Context,
    mods: Vec<Arc<RwLock<EngineMod>>>,
    new_mod_path: String,
    pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
}

impl ModManager {
    pub fn event(&mut self, ev: WindowEvent) -> anyhow::Result<()> {
        if let WindowEvent::KeyboardInput { event, .. } = ev {
            match event.physical_key {
                winit::keyboard::PhysicalKey::Code(key_code) => {
                    let key_code = key_code as u32;
                    let key_code: KeyCode = KeyCode::try_from(key_code).unwrap();
                    for engine_mod in self.mods.iter() {
                        let mut mod_lock = engine_mod.write().unwrap();
                        mod_lock.key_event(key_code.clone())?;
                    }
                }
                winit::keyboard::PhysicalKey::Unidentified(_) => {}
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
            )?)));
        }
        for engine_mod in self.mods.iter() {
            let mut mod_lock = engine_mod.write().unwrap();
            mod_lock.update().unwrap();
        }
        Ok(())
    }
    pub fn new(gui_context: Context, pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>) -> Self {
        let engine = Engine::default();
        let test_mod = Arc::new(RwLock::new(
            EngineMod::new(
                "./target/wasm32-unknown-unknown/release/example_mod.wasm".to_string(),
                &engine,
                gui_context.clone(),
                pressed_keys_buffer.clone(),
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
        }
    }
}
