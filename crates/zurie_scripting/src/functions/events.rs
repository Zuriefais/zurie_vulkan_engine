use crate::engine_mod::EngineMod;
use crate::utils::{get_bytes_from_wasm, get_string_by_ptr};
use hashbrown::HashMap;
use log::info;
use std::sync::{Arc, RwLock};
use wasmtime::{Linker, Store};
use zurie_shared::slotmap::{Key, SlotMap};

use zurie_shared::slotmap::{self, DefaultKey, KeyData};

#[derive(Clone, Default)]
pub struct EventManager {
    pub event_storage: Arc<RwLock<SlotMap<DefaultKey, String>>>,
    pub event_queue: Arc<RwLock<Vec<(u64, Vec<u8>)>>>,
    pub subscribed_mods: Arc<RwLock<SlotMap<DefaultKey, EngineMod>>>,
}

impl EventManager {
    fn add_mod(&mut self) {}

    fn subscribe_by_name() {}
    fn subscribe_by_handle(&mut self, event_handle: u64) {}
    fn send_event(&mut self, event_handle: DefaultKey, data: &[u8], mod_handle: DefaultKey) {
        info!(
            "Event {:?} sended by {}",
            self.event_storage.read().unwrap().get(event_handle),
            self.subscribed_mods
                .read()
                .unwrap()
                .get(mod_handle)
                .unwrap()
                .mod_name
                .read()
                .unwrap()
        );
    }
}

pub fn register_events_bindings(linker: &mut Linker<()>, store: &Store<()>) -> anyhow::Result<()> {
    register_subscribe_to_event_by_name(linker, store)?;
    Ok(())
}

fn register_subscribe_to_event_by_name(
    linker: &mut Linker<()>,
    store: &Store<()>,
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
            // let name = get_string_by_ptr(
            //     &mut caller,
            //     params[0].unwrap_i32() as u32,
            //     params[1].unwrap_i32() as u32,
            // )?;
            // info!("Event registered: {}", name);

            // let mut storage_lock = event_storage.write().unwrap();
            // let mut name_lock = event_name_map.write().unwrap();
            // let mut subscribed_lock = subscribed_mods.write().unwrap();

            // let index = match name_lock.get(&name) {
            //     Some(&existing_key) => existing_key,
            //     None => {
            //         let new_key = storage_lock.insert(());
            //         name_lock.insert(name, new_key);
            //         subscribed_lock[mod_index].push(new_key);
            //         new_key
            //     }
            // };

            // results[0] = wasmtime::Val::I64(KeyData::as_ffi(index.data()) as i64);
            Ok(())
        },
    )?;
    Ok(())
}

// extern "C" {
//     fn subscribe_to_event_by_name_sys(ptr: u32, len: u32) -> u64;
//     fn subscribe_to_event_by_handle_sys(handle: u64);
//     fn send_event_sys(handle: u64, ptr: u32, len: u32);
// }
