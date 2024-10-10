use crate::functions::{
    gui::{register_gui_button, register_gui_text},
    input::{register_key_pressed, register_subscribe_for_key_event_sys},
    utils::{register_get_delta_time, register_get_mod_name_callback, register_info},
};
use anyhow::Ok;
use egui_winit_vulkano::egui::Context;
use hashbrown::HashSet;
use log::info;
use shared_types::KeyCode;
use std::sync::{Arc, RwLock};
use wasmtime::{Engine, Instance, Linker, Module, Store, TypedFunc};

#[derive()]
pub struct EngineMod {
    pub path: String,
    pub module: Module,
    pub instance: Instance,
    pub store: Store<()>,
    pub update_fn: TypedFunc<(), ()>,
    pub key_event_fn: TypedFunc<u32, ()>,
    pub mod_name: Arc<RwLock<String>>,
    pub subscribed_keys: Arc<RwLock<HashSet<KeyCode>>>,
}

impl EngineMod {
    pub fn new(
        mod_path: String,
        engine: &Engine,
        gui_context: Context,
        pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
    ) -> anyhow::Result<Self> {
        let mut linker: Linker<()> = Linker::new(engine);
        let mod_name = Arc::new(RwLock::new("No name".to_string()));
        let module = Module::from_file(engine, &mod_path)?;
        let subscribed_keys: Arc<RwLock<HashSet<KeyCode>>> = Default::default();
        info!("mod at path {} compiled", mod_path);
        let mut store = Store::new(engine, ());
        register_get_delta_time(&mut linker)?;
        register_info(&mut linker, mod_name.clone())?;
        register_gui_text(&mut linker, gui_context.clone())?;
        register_get_mod_name_callback(&mut linker, mod_name.clone())?;
        register_subscribe_for_key_event_sys(
            &mut linker,
            mod_name.clone(),
            subscribed_keys.clone(),
        )?;
        register_gui_button(&mut linker, &store, gui_context.clone())?;
        register_key_pressed(&mut linker, pressed_keys_buffer, &store)?;

        let instance = linker.instantiate(&mut store, &module)?;
        let init_fn: TypedFunc<(), ()> = instance.get_typed_func::<(), ()>(&mut store, "init")?;
        let update_fn: TypedFunc<(), ()> =
            instance.get_typed_func::<(), ()>(&mut store, "update")?;
        let key_event_fn: TypedFunc<u32, ()> =
            instance.get_typed_func::<u32, ()>(&mut store, "key_event")?;
        let get_mod_name_fn: TypedFunc<(), ()> =
            instance.get_typed_func::<(), ()>(&mut store, "get_mod_name")?;
        get_mod_name_fn.call(&mut store, ())?;
        info!("Mod name: {}", mod_name.read().unwrap());
        init_fn.call(&mut store, ())?;
        Ok(Self {
            path: mod_path,
            module,
            instance,
            store,
            update_fn,
            key_event_fn,
            mod_name,
            subscribed_keys,
        })
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        self.update_fn.call(&mut self.store, ())?;
        Ok(())
    }

    pub fn key_event(&mut self, key_code: KeyCode) -> anyhow::Result<()> {
        let keys_lock = self.subscribed_keys.read().unwrap();
        if keys_lock.contains(&key_code) {
            self.key_event_fn.call(&mut self.store, key_code as u32)?;
        }
        Ok(())
    }
}
