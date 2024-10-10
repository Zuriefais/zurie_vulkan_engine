use anyhow::Ok;
use egui_winit_vulkano::egui::{self, Context};
use hashbrown::HashSet;
use log::info;
use shared_types::{GuiTextMessage, KeyCode};
use std::sync::{Arc, RwLock};
use wasmtime::{Caller, Engine, Instance, Linker, Module, Store, TypedFunc};

pub fn register_subscribe_for_key_event_sys(
    linker: &mut Linker<()>,
    mod_name: Arc<RwLock<String>>,
    subscribed_keys: Arc<RwLock<HashSet<KeyCode>>>,
) -> anyhow::Result<()> {
    linker.func_wrap("env", "subscribe_for_key_event_sys", move |key: u32| {
        let key: KeyCode = KeyCode::try_from(key).unwrap();
        info!(target: mod_name.read().unwrap().as_str(), "subscribed for {:?}", key);
        let mut keys_lock = subscribed_keys.write().unwrap();
        keys_lock.insert(key);
    })?;
    Ok(())
}

pub fn register_key_pressed(
    linker: &mut Linker<()>,
    pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
    store: &Store<()>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "key_pressed_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32].iter().cloned(),
            [wasmtime::ValType::I32].iter().cloned(),
        ),
        move |_, params, results| {
            let key: KeyCode = KeyCode::try_from(params[0].unwrap_i32() as u32).unwrap();
            let clicked = pressed_keys_buffer.read().unwrap().contains(&key) as i32;
            results[0] = wasmtime::Val::I32(clicked);
            Ok(())
        },
    )?;
    Ok(())
}
