// use anyhow::Ok;
// use hashbrown::HashSet;
// use log::info;
// use std::sync::{Arc, RwLock};
// use wasmtime::{Caller, Linker, Store};
// use zurie_types::{glam::Vec2, KeyCode, Vector2};

// use crate::utils::copy_obj_to_memory;

// pub fn register_subscribe_for_key_event(
//     linker: &mut Linker<()>,
//     mod_name: Arc<RwLock<String>>,
//     subscribed_keys: Arc<RwLock<HashSet<KeyCode>>>,
// ) -> anyhow::Result<()> {
//     linker.func_wrap("env", "subscribe_for_key_event_sys", move |key: u32| {
//         let key: KeyCode = KeyCode::try_from(key).unwrap();
//         info!(target: mod_name.read().unwrap().as_str(), "subscribed for {:?}", key);
//         let mut keys_lock = subscribed_keys.write().unwrap();
//         keys_lock.insert(key);
//     })?;
//     Ok(())
// }

// pub fn register_key_pressed(
//     linker: &mut Linker<()>,
//     pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
//     store: &Store<()>,
// ) -> anyhow::Result<()> {
//     linker.func_new(
//         "env",
//         "key_pressed_sys",
//         wasmtime::FuncType::new(
//             store.engine(),
//             [wasmtime::ValType::I32].iter().cloned(),
//             [wasmtime::ValType::I32].iter().cloned(),
//         ),
//         move |_, params, results| {
//             let key: KeyCode = KeyCode::try_from(params[0].unwrap_i32() as u32).unwrap();
//             let clicked = pressed_keys_buffer.read().unwrap().contains(&key) as i32;
//             results[0] = wasmtime::Val::I32(clicked);
//             Ok(())
//         },
//     )?;
//     Ok(())
// }

// pub fn register_request_mouse_pos(
//     linker: &mut Linker<()>,
//     mouse_pos: Arc<RwLock<Vec2>>,
// ) -> anyhow::Result<()> {
//     let mouse_pos = mouse_pos.clone();
//     linker.func_wrap(
//         "env",
//         "request_mouse_pos_sys",
//         move |mut caller: Caller<'_, ()>| {
//             let alloc_fn = caller
//                 .get_export("alloc")
//                 .and_then(|export| export.into_func())
//                 .ok_or_else(|| anyhow::anyhow!("Failed to find 'alloc' function"))?
//                 .typed::<u32, u32>(&caller)?;
//             let vec2 = mouse_pos.read().unwrap();
//             let vector2 = Vector2::from(*vec2);
//             Ok(copy_obj_to_memory(&mut caller, vector2, alloc_fn.clone()))
//         },
//     )?;
//     Ok(())
// }
