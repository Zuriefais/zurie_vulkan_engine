use log::{info, warn};
use std::sync::{Arc, RwLock};
use wasmtime::{Linker, Store};
use zurie_types::Object;

use crate::utils::{copy_obj_to_memory, get_obj_by_ptr};

pub fn register_game_logic_bindings(
    linker: &mut Linker<()>,
    store: &Store<()>,
    object_storage: Arc<RwLock<Vec<Object>>>,
) -> anyhow::Result<()> {
    register_spawn_object(linker, store, object_storage.clone())?;
    register_request_object(linker, store, object_storage.clone())?;
    register_request_object_position(linker, store, object_storage.clone())?;
    register_set_object_position(linker, store, object_storage.clone())?;
    Ok(())
}

pub fn register_spawn_object(
    linker: &mut Linker<()>,
    store: &Store<()>,
    object_storage: Arc<RwLock<Vec<Object>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "spawn_object_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [wasmtime::ValType::I32].iter().cloned(),
        ),
        move |mut caller, params, results| {
            let obj = get_obj_by_ptr::<Object>(
                &mut caller,
                params[0].unwrap_i32() as u32,
                params[1].unwrap_i32() as u32,
            )?;
            let mut storage_lock = object_storage.write().unwrap();
            info!("Object spawned. obj: {:?}", &obj);
            storage_lock.push(obj);
            let index = (storage_lock.len() - 1) as i32;

            results[0] = wasmtime::Val::I32(index);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_request_object(
    linker: &mut Linker<()>,
    store: &Store<()>,
    object_storage: Arc<RwLock<Vec<Object>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "request_object_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32].iter().cloned(),
            [wasmtime::ValType::I32].iter().cloned(),
        ),
        move |mut caller, params, results| {
            let index = params[0].unwrap_i32() as usize;

            let storage_lock = object_storage.read().unwrap();
            let object: Option<&Object> = storage_lock.get(index);
            if let Some(obj) = object {
                let obj_clone = obj.clone();
                let alloc_fn = caller
                    .get_export("alloc")
                    .and_then(|export| export.into_func())
                    .ok_or_else(|| anyhow::anyhow!("Failed to find 'alloc' function"))?
                    .typed::<u32, u32>(&caller)?;
                copy_obj_to_memory(&mut caller, obj_clone, alloc_fn.clone())?;
                info!("Object requested: {}. obj: {:?}", index, &obj);
            } else {
                warn!("Object requested by: {} can't find", index,);
            }

            results[0] = wasmtime::Val::I32(object.is_none() as i32);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_request_object_position(
    linker: &mut Linker<()>,
    store: &Store<()>,
    object_storage: Arc<RwLock<Vec<Object>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "request_object_position_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32].iter().cloned(),
            [wasmtime::ValType::I32].iter().cloned(),
        ),
        move |mut caller, params, results| {
            let index = params[0].unwrap_i32() as usize;

            let storage_lock = object_storage.read().unwrap();
            let object: Option<&Object> = storage_lock.get(index);
            if let Some(obj) = object {
                let obj_clone = obj.position.clone();
                let alloc_fn = caller
                    .get_export("alloc")
                    .and_then(|export| export.into_func())
                    .ok_or_else(|| anyhow::anyhow!("Failed to find 'alloc' function"))?
                    .typed::<u32, u32>(&caller)?;
                copy_obj_to_memory(&mut caller, obj_clone, alloc_fn.clone())?;
            }

            results[0] = wasmtime::Val::I32(object.is_none() as i32);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_set_object_position(
    linker: &mut Linker<()>,
    store: &Store<()>,
    object_storage: Arc<RwLock<Vec<Object>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "set_object_position_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [].iter().cloned(),
        ),
        move |mut caller, params, _| {
            let (index, ptr, len) = (
                params[0].unwrap_i32() as usize,
                params[1].unwrap_i32() as u32,
                params[2].unwrap_i32() as u32,
            );
            let mut storage_lock = object_storage.write().unwrap();
            let new_position = get_obj_by_ptr(&mut caller, ptr, len)?;
            let object: Option<&mut Object> = storage_lock.get_mut(index);
            if let Some(obj) = object {
                obj.position = new_position;
            }
            Ok(())
        },
    )?;
    Ok(())
}
