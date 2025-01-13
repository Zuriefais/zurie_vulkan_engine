// use crate::utils::{get_bytes_from_wasm, get_string_by_ptr};
// use anyhow::Ok;
// use std::{
//     path::Path,
//     sync::{Arc, RwLock},
// };
// use zurie_render::sprite::{LoadSpriteInfo, SpriteManager};
// use zurie_shared::slotmap::Key;
// use zurie_shared::slotmap::KeyData;
// use zurie_types::SpriteHandle;

// use wasmtime::{Linker, Store, Val};

// pub fn setup_sprite_bindings(
//     linker: &mut Linker<()>,
//     store: &Store<()>,
//     sprite_manager: Arc<RwLock<SpriteManager>>,
// ) -> anyhow::Result<()> {
//     register_load_sprite_from_file(linker, store, sprite_manager.clone())?;
//     register_load_sprite_from_buffer(linker, store, sprite_manager.clone())?;
//     Ok(())
// }

// pub fn register_load_sprite_from_file(
//     linker: &mut Linker<()>,
//     store: &Store<()>,
//     sprite_manager: Arc<RwLock<SpriteManager>>,
// ) -> anyhow::Result<()> {
//     linker.func_new(
//         "env",
//         "load_sprite_from_file_sys",
//         wasmtime::FuncType::new(
//             store.engine(),
//             [wasmtime::ValType::I32, wasmtime::ValType::I32]
//                 .iter()
//                 .cloned(),
//             [wasmtime::ValType::I64].iter().cloned(),
//         ),
//         move |mut caller, params, results| -> anyhow::Result<()> {
//             let path = get_string_by_ptr(
//                 &mut caller,
//                 params[0].unwrap_i32() as u32,
//                 params[1].unwrap_i32() as u32,
//             )?;

//             let handle = sprite_manager
//                 .write()
//                 .unwrap()
//                 .push_to_load_queue(LoadSpriteInfo::Path(Box::from(Path::new(&path))));

//             results[0] = Val::I64(handle.data().as_ffi() as i64);
//             Ok(())
//         },
//     )?;
//     Ok(())
// }

// pub fn register_load_sprite_from_buffer(
//     linker: &mut Linker<()>,
//     store: &Store<()>,
//     sprite_manager: Arc<RwLock<SpriteManager>>,
// ) -> anyhow::Result<()> {
//     linker.func_new(
//         "env",
//         "load_sprite_from_buffer_sys",
//         wasmtime::FuncType::new(
//             store.engine(),
//             [wasmtime::ValType::I32, wasmtime::ValType::I32]
//                 .iter()
//                 .cloned(),
//             [wasmtime::ValType::I64].iter().cloned(),
//         ),
//         move |mut caller, params, results| -> anyhow::Result<()> {
//             let buffer = get_bytes_from_wasm(
//                 &mut caller,
//                 params[0].unwrap_i32() as u32,
//                 params[1].unwrap_i32() as u32,
//             )?;

//             let handle = sprite_manager
//                 .write()
//                 .unwrap()
//                 .push_to_load_queue(LoadSpriteInfo::Buffer(buffer));
//             results[0] = Val::I64(handle.data().as_ffi() as i64);
//             Ok(())
//         },
//     )?;
//     Ok(())
// }

// pub fn register_get_sprite_width(
//     linker: &mut Linker<()>,
//     store: &Store<()>,
//     sprite_manager: Arc<RwLock<SpriteManager>>,
// ) -> anyhow::Result<()> {
//     linker.func_new(
//         "env",
//         "get_sprite_width_sys",
//         wasmtime::FuncType::new(
//             store.engine(),
//             [wasmtime::ValType::I64].iter().cloned(),
//             [wasmtime::ValType::I32].iter().cloned(),
//         ),
//         move |_, params, results| -> anyhow::Result<()> {
//             let handle: SpriteHandle = get_handle(params);
//             let width = match sprite_manager.read().unwrap().get_sprite(handle) {
//                 Some(it) => it,
//                 None => anyhow::bail!("could't get sprite: {:?} width", handle),
//             }
//             .width;
//             results[0] = Val::I32(width as i32);
//             Ok(())
//         },
//     )?;
//     Ok(())
// }

// pub fn register_get_sprite_height(
//     linker: &mut Linker<()>,
//     store: &Store<()>,
//     sprite_manager: Arc<RwLock<SpriteManager>>,
// ) -> anyhow::Result<()> {
//     linker.func_new(
//         "env",
//         "get_sprite_height_sys",
//         wasmtime::FuncType::new(
//             store.engine(),
//             [wasmtime::ValType::I64].iter().cloned(),
//             [wasmtime::ValType::I32].iter().cloned(),
//         ),
//         move |_, params, results| -> anyhow::Result<()> {
//             let handle: SpriteHandle = get_handle(params);
//             let height = match sprite_manager.read().unwrap().get_sprite(handle) {
//                 Some(it) => it,
//                 None => anyhow::bail!("could't get sprite: {:?} height", handle),
//             }
//             .height;
//             results[0] = Val::I32(height as i32);
//             Ok(())
//         },
//     )?;
//     Ok(())
// }

// fn get_handle(params: &[Val]) -> SpriteHandle {
//     KeyData::from_ffi(params[0].unwrap_i64() as u64).into()
// }
