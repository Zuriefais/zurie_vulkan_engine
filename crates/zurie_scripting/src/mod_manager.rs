use anyhow::Ok;
use egui::{self, Context};
use hashbrown::HashSet;
use log::{error, info};
use std::sync::{Arc, RwLock};
use wasmtime::Engine;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
use winit::{
    dpi::PhysicalPosition,
    event::{MouseScrollDelta, WindowEvent},
};
use zurie_audio::AudioManager;
use zurie_ecs::World;
use zurie_render::sprite::SpriteManager;
use zurie_shared::slotmap::SlotMap;
use zurie_types::{camera::Camera, glam::Vec2, KeyCode};

use crate::{functions::events::EventManager, ModHandle};

use super::engine_mod::EngineMod;

pub struct ModManager {
    engine: Engine,
    gui_context: Context,
    mods: SlotMap<ModHandle, Arc<RwLock<EngineMod>>>,
    new_mod_path: String,
    pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
    mouse_pos: Arc<RwLock<Vec2>>,
    world: Arc<RwLock<World>>,
    camera: Arc<RwLock<Camera>>,
    event_manager: Arc<RwLock<EventManager>>,
    sprite_manager: Arc<RwLock<SpriteManager>>,
    audio_manager: AudioManager,
    #[cfg(target_os = "android")]
    app: AndroidApp,
}

impl ModManager {
    pub fn window_event(&mut self, ev: WindowEvent) -> anyhow::Result<()> {
        if let WindowEvent::KeyboardInput { event, .. } = ev.clone() {
            match event.physical_key {
                winit::keyboard::PhysicalKey::Code(key_code) => {
                    let key_code = key_code as u32;
                    let key_code: KeyCode = KeyCode::try_from(key_code).unwrap();
                    for (_, engine_mod) in self.mods.iter() {
                        let mut mod_lock = engine_mod.write().unwrap();
                        mod_lock.key_event(key_code)?;
                    }
                }
                winit::keyboard::PhysicalKey::Unidentified(_) => {}
            }
        }
        if let WindowEvent::MouseWheel { delta, .. } = ev {
            let scroll_amount: f32 = match delta {
                MouseScrollDelta::LineDelta(_, y) => y,
                MouseScrollDelta::PixelDelta(pos) => {
                    if pos == PhysicalPosition::new(-0.0, -0.0) {
                        0.0
                    } else {
                        -(pos.y as f32)
                    }
                }
            };
            if scroll_amount == 0.0 {
                return Ok(());
            }
            for (_, engine_mod) in self.mods.iter() {
                let mut mod_lock = engine_mod.write().unwrap();
                mod_lock.scroll(scroll_amount)?;
            }
        }
        Ok(())
    }
    fn gui(&mut self) -> anyhow::Result<(bool, bool)> {
        let mut reload_mods = false;
        let mut load_new_mod = false;
        egui::Window::new("Mods Window").show(&self.gui_context, |ui| {
            reload_mods = ui.button("reload mods").clicked();
            load_new_mod = ui.button("Load new mod").clicked();
            ui.label("mod path:");
            ui.text_edit_singleline(&mut self.new_mod_path);
            ui.label("Loaded mods:");
            for (_, loaded_mod) in self.mods.iter() {
                ui.label(format!(
                    "path: {}, name: {}",
                    loaded_mod.read().unwrap().path,
                    loaded_mod.read().unwrap().mod_name.read().unwrap()
                ));
            }
        });
        Ok((reload_mods, load_new_mod))
    }

    fn load_mod(
        &self,
        mod_path: &String,
        handle: ModHandle,
    ) -> anyhow::Result<Arc<RwLock<EngineMod>>> {
        #[cfg(target_os = "android")]
        return Ok(Arc::new(RwLock::new(EngineMod::new(
            mod_path.clone(),
            &self.engine,
            self.gui_context.clone(),
            self.pressed_keys_buffer.clone(),
            self.mouse_pos.clone(),
            self.world.clone(),
            self.camera.clone(),
            self.event_manager.clone(),
            handle,
            self.app.clone(),
        )?)));
        #[cfg(not(target_os = "android"))]
        Ok(Arc::new(RwLock::new(EngineMod::new(
            mod_path.clone(),
            &self.engine,
            self.gui_context.clone(),
            self.pressed_keys_buffer.clone(),
            self.mouse_pos.clone(),
            self.world.clone(),
            self.camera.clone(),
            self.event_manager.clone(),
            handle,
            self.sprite_manager.clone(),
            self.audio_manager.clone(),
        )?)))
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        let (reload_mods, load_new_mod) = self.gui()?;
        if reload_mods {
            let mod_paths: Vec<(ModHandle, String)> = self
                .mods
                .iter()
                .map(|(handle, engine_mod)| {
                    let path = engine_mod.read().unwrap().path.clone();
                    (handle, path)
                })
                .collect();
            for (handle, mod_path) in mod_paths {
                let new_mod = self.load_mod(&mod_path, handle)?;

                if let Some(engine_mod) = self.mods.get_mut(handle) {
                    *engine_mod = new_mod;
                }
                info!("reloading {}", mod_path);
            }
        }
        if load_new_mod {
            info!("Loading mod at path: {}", self.new_mod_path.clone());
            let event_manager = self.event_manager.clone();

            self.mods.insert_with_key(|handle| {
                Arc::new(RwLock::new(
                    EngineMod::new(
                        self.new_mod_path.clone(),
                        &self.engine,
                        self.gui_context.clone(),
                        self.pressed_keys_buffer.clone(),
                        self.mouse_pos.clone(),
                        self.world.clone(),
                        self.camera.clone(),
                        event_manager,
                        handle,
                        self.sprite_manager.clone(),
                        self.audio_manager.clone(),
                        #[cfg(target_os = "android")]
                        self.app.clone(),
                    )
                    .unwrap(),
                ))
            });
        }
        self.event_manager
            .write()
            .unwrap()
            .process_events(&mut self.mods)?;
        for (_, engine_mod) in self.mods.iter() {
            let mut mod_lock = engine_mod.write().unwrap();
            if let Err(e) = mod_lock.update() {
                error!("Error updating mod {}: {}", mod_lock.path, e);
                continue; // Skip this mod but continue with others
            }
        }
        Ok(())
    }
    pub fn new(
        gui_context: Context,
        pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
        mouse_pos: Arc<RwLock<Vec2>>,
        world: Arc<RwLock<World>>,
        camera: Arc<RwLock<Camera>>,
        sprite_manager: Arc<RwLock<SpriteManager>>,
        #[cfg(target_os = "android")] android_app: AndroidApp,
    ) -> Self {
        let engine = Engine::default();
        let mut mods = SlotMap::with_key();
        let event_manager: Arc<RwLock<EventManager>> = Default::default();
        let audio_manager = AudioManager::new();
        #[cfg(not(target_os = "android"))]
        mods.insert_with_key(|handle| {
            Arc::new(RwLock::new(
                EngineMod::new(
                    "./target/wasm32-unknown-unknown/release/vampire_like_demo.wasm".into(),
                    &engine,
                    gui_context.clone(),
                    pressed_keys_buffer.clone(),
                    mouse_pos.clone(),
                    world.clone(),
                    camera.clone(),
                    event_manager.clone(),
                    handle,
                    sprite_manager.clone(),
                    audio_manager.clone(),
                )
                .unwrap(),
            ))
        });
        #[cfg(target_os = "android")]
        mods.insert_with_key(|handle| {
            Arc::new(RwLock::new(
                EngineMod::new(
                    "example_mod.wasm".into(),
                    &engine,
                    gui_context.clone(),
                    pressed_keys_buffer.clone(),
                    mouse_pos.clone(),
                    world.clone(),
                    camera.clone(),
                    event_manager.clone(),
                    handle,
                    android_app.clone(),
                )
                .unwrap(),
            ))
        });

        Self {
            engine,
            gui_context,
            mods,
            new_mod_path: String::new(),
            pressed_keys_buffer,
            mouse_pos,
            world,
            camera,
            event_manager,
            sprite_manager,
            audio_manager,
            #[cfg(target_os = "android")]
            app: android_app,
        }
    }
}
