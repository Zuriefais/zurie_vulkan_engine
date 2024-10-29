use crate::{
    functions::{
        camera::register_camera_bindings,
        events::{register_events_bindings, EventHandle, EventManager},
        file::register_file_bindings,
        game_logic::register_game_logic_bindings,
        gui::{register_gui_button, register_gui_text},
        input::{
            register_key_pressed, register_request_mouse_pos, register_subscribe_for_key_event,
        },
        utils::register_utils_bindings,
    },
    mod_manager::ModHandle,
};
use anyhow::Ok;
use egui::Context;
use hashbrown::HashSet;
use log::info;
use std::sync::{Arc, RwLock};
use wasmtime::AsContextMut;
use wasmtime::{Engine, Instance, Linker, Module, Store, TypedFunc};
use zurie_shared::slotmap::{DefaultKey, Key, KeyData, SlotMap};
use zurie_types::{camera::Camera, glam::Vec2, KeyCode, Object};

pub struct EngineMod {
    pub path: String,
    pub module: Module,
    pub instance: Instance,
    pub store: Store<()>,
    pub update_fn: TypedFunc<(), ()>,
    pub event_fn: TypedFunc<u64, ()>,
    pub key_event_fn: TypedFunc<u32, ()>,
    pub scroll_fn: TypedFunc<f32, ()>,
    pub mod_name: Arc<RwLock<String>>,
    pub alloc_fn: TypedFunc<u32, u32>,
    pub subscribed_keys: Arc<RwLock<HashSet<KeyCode>>>,
}

impl EngineMod {
    pub fn new(
        mod_path: String,
        engine: &Engine,
        gui_context: Context,
        pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
        mouse_pos: Arc<RwLock<Vec2>>,
        object_storage: Arc<RwLock<SlotMap<DefaultKey, Object>>>,
        camera: Arc<RwLock<Camera>>,
        event_manager: Arc<RwLock<EventManager>>,
        mod_handle: ModHandle,
    ) -> anyhow::Result<Self> {
        let mut linker: Linker<()> = Linker::new(engine);
        let mod_name = Arc::new(RwLock::new("No name".to_string()));
        let module = Module::from_file(engine, &mod_path)?;
        let subscribed_keys: Arc<RwLock<HashSet<KeyCode>>> = Default::default();
        info!("mod at path {} compiled", mod_path);
        let mut store = Store::new(engine, ());
        let alloc_fn_lock = Arc::new(RwLock::new(None));
        register_utils_bindings(&mut linker, &store, mod_name.clone())?;
        register_gui_text(&mut linker, gui_context.clone())?;
        register_subscribe_for_key_event(&mut linker, mod_name.clone(), subscribed_keys.clone())?;
        register_gui_button(&mut linker, &store, gui_context.clone())?;
        register_key_pressed(&mut linker, pressed_keys_buffer, &store)?;
        register_request_mouse_pos(&mut linker, mouse_pos)?;
        register_game_logic_bindings(&mut linker, &store, object_storage)?;
        register_camera_bindings(&mut linker, camera, &store)?;
        register_events_bindings(&mut linker, &store, event_manager, mod_handle)?;
        register_file_bindings(&mut linker, &store, alloc_fn_lock.clone())?;
        let instance = linker.instantiate(&mut store, &module)?;
        let new_fn: TypedFunc<(), ()> = instance.get_typed_func::<(), ()>(&mut store, "new")?;
        let init_fn: TypedFunc<(), ()> = instance.get_typed_func::<(), ()>(&mut store, "init")?;
        let update_fn: TypedFunc<(), ()> =
            instance.get_typed_func::<(), ()>(&mut store, "update")?;
        let key_event_fn: TypedFunc<u32, ()> =
            instance.get_typed_func::<u32, ()>(&mut store, "key_event")?;
        let scroll_fn: TypedFunc<f32, ()> =
            instance.get_typed_func::<f32, ()>(&mut store, "scroll")?;
        let get_mod_name_fn: TypedFunc<(), ()> =
            instance.get_typed_func::<(), ()>(&mut store, "get_mod_name")?;
        let event_fn: TypedFunc<u64, ()> =
            instance.get_typed_func::<u64, ()>(&mut store, "event")?;
        let alloc_fn = instance.get_typed_func::<u32, u32>(&mut store, "alloc")?;
        let mut alloc_fn_lock = alloc_fn_lock.write().unwrap();
        *alloc_fn_lock = Some(alloc_fn.clone());
        new_fn.call(&mut store, ())?;
        get_mod_name_fn.call(&mut store, ())?;
        info!("Mod name: {}", mod_name.read().unwrap());
        init_fn.call(&mut store, ())?;
        Ok(Self {
            path: mod_path,
            module,
            instance,
            store,
            update_fn,
            event_fn,
            key_event_fn,
            scroll_fn,
            mod_name,
            alloc_fn,
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

    pub fn scroll(&mut self, scroll: f32) -> anyhow::Result<()> {
        self.scroll_fn.call(&mut self.store, scroll)?;
        Ok(())
    }

    pub fn handle_event(&mut self, event_handle: EventHandle, data: &[u8]) -> anyhow::Result<()> {
        let mut store_mut = self.store.as_context_mut();
        let memory = self
            .instance
            .get_memory(&mut store_mut, "memory")
            .ok_or_else(|| anyhow::anyhow!("failed to get memory"))?;
        let ptr = self.alloc_fn.call(&mut self.store, data.len() as u32)? as usize;
        memory
            .data_mut(&mut self.store)
            .get_mut(ptr..)
            .and_then(|slice| slice.get_mut(..data.len()))
            .ok_or_else(|| anyhow::anyhow!("failed to write to memory"))?
            .copy_from_slice(data);
        self.event_fn
            .call(&mut self.store, KeyData::as_ffi(event_handle.data()))?;
        Ok(())
    }
}
