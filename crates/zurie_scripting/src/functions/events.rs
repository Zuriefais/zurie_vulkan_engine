use crate::engine_mod::{EngineMod};
use crate::mod_manager::ModHandle;
use crate::utils::{get_bytes_from_wasm, get_string_by_ptr};
use egui::ahash::{HashSet, HashSetExt};
use hashbrown::HashMap;
use log::info;
use std::sync::{Arc, RwLock};
use wasmtime::{Linker, Store};
use zurie_shared::slotmap::{new_key_type, Key, KeyData, SlotMap};
new_key_type! { pub struct EventHandle; }

#[derive(Clone, Default)]
pub struct EventManager {
    pub event_storage: SlotMap<EventHandle, String>,
    pub event_handlers: HashMap<ModHandle, HashSet<EventHandle>>,
    pub event_queue: Vec<(ModHandle, EventHandle, Vec<u8>)>,
}

impl EventManager {
    pub fn subscribe_by_handle(&mut self, event_handle: EventHandle, mod_handle: ModHandle) {
        match self.event_handlers.get_mut(&mod_handle) {
            Some(subscribed) => {
                subscribed.insert(event_handle);
            }
            None => {
                self.event_handlers.insert(mod_handle, {
                    let mut subscribed = HashSet::new();
                    subscribed.insert(event_handle);
                    subscribed
                });
            }
        }
    }
    pub fn subscribe_by_name(&mut self, name: String, mod_handle: ModHandle) -> EventHandle {
        let event_handle = self
            .event_storage
            .iter()
            .find(|(_, event_name)| name == **event_name)
            .map(|(key, _)| key)
            .unwrap_or_else(|| self.event_storage.insert(name.clone()));
        self.subscribe_by_handle(event_handle, mod_handle);
        info!("Event registered: {}", name);
        event_handle
    }
    pub fn emit(&mut self, event_handle: EventHandle, mod_handle: ModHandle, data: Vec<u8>) {
        self.event_queue.push((mod_handle, event_handle, data))
    }
    pub fn process_events(
        &mut self,
        mods: &mut SlotMap<ModHandle, Arc<RwLock<EngineMod>>>,
    ) -> anyhow::Result<()> {
        for (_, event_handle, data) in self.event_queue.drain(..) {
            for (_, engine_mod) in mods.iter() {
                engine_mod
                    .write()
                    .unwrap()
                    .handle_event(event_handle, &data)?;
            }
        }
        Ok(())
    }
}

pub fn register_events_bindings(
    linker: &mut Linker<()>,
    store: &Store<()>,
    event_manager: Arc<RwLock<EventManager>>,
    mod_handle: ModHandle,
) -> anyhow::Result<()> {
    register_subscribe_to_event_by_name(linker, store, event_manager.clone(), mod_handle)?;
    register_subscribe_to_event_by_handle(
        linker,
        store,
        event_manager.clone(),
        mod_handle,
    )?;
    register_emit_event(linker, store, event_manager.clone(), mod_handle)?;
    Ok(())
}

fn register_subscribe_to_event_by_name(
    linker: &mut Linker<()>,
    store: &Store<()>,
    event_manager: Arc<RwLock<EventManager>>,
    mod_handle: ModHandle,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "subscribe_to_event_by_name_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [wasmtime::ValType::I64].iter().cloned(),
        ),
        move |mut caller, params, results| {
            let name = get_string_by_ptr(
                &mut caller,
                params[0].unwrap_i32() as u32,
                params[1].unwrap_i32() as u32,
            )?;

            let mut event_manager = event_manager.write().unwrap();
            let handle = event_manager.subscribe_by_name(name, mod_handle);

            results[0] = wasmtime::Val::I64(KeyData::as_ffi(handle.data()) as i64);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_subscribe_to_event_by_handle(
    linker: &mut Linker<()>,
    store: &Store<()>,
    event_manager: Arc<RwLock<EventManager>>,
    mod_handle: ModHandle,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "subscribe_to_event_by_handle",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I64].iter().cloned(),
            [].iter().cloned(),
        ),
        move |_, params, _| {
            let handle = KeyData::from_ffi(params[0].unwrap_i64() as u64);

            let mut event_manager = event_manager.write().unwrap();
            event_manager.subscribe_by_handle(handle.into(), mod_handle);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_emit_event(
    linker: &mut Linker<()>,
    store: &Store<()>,
    event_manager: Arc<RwLock<EventManager>>,
    mod_handle: ModHandle,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "emit_event_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [
                wasmtime::ValType::I64,
                wasmtime::ValType::I32,
                wasmtime::ValType::I32,
            ]
            .iter()
            .cloned(),
            [].iter().cloned(),
        ),
        move |mut caller, params, _| {
            let handle = KeyData::from_ffi(params[0].unwrap_i64() as u64);
            let (ptr, len) = (params[1].unwrap_i32() as u32, params[2].unwrap_i32() as u32);
            let data = get_bytes_from_wasm(&mut caller, ptr, len)?;
            let mut event_manager = event_manager.write().unwrap();
            info!(
                "Event emited: {}",
                event_manager.event_storage.get(handle.into()).unwrap()
            );
            event_manager.emit(handle.into(), mod_handle, data);
            Ok(())
        },
    )?;
    Ok(())
}

// extern "C" {
//     fn subscribe_to_event_by_name_sys(ptr: u32, len: u32) -> u64;
//     fn subscribe_to_event_by_handle_sys(handle: u64);
//     fn emit_sys(handle: u64, ptr: u32, len: u32);
// }
