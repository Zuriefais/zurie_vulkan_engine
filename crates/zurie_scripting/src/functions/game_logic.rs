use anyhow::Ok;
use log::{info, warn};
use std::sync::{Arc, RwLock};
use wasmtime::{Linker, Store, TypedFunc};
use zurie_shared::slotmap::{DefaultKey, Key, KeyData, SlotMap};
use zurie_types::Object;

use crate::utils::{copy_obj_to_memory, get_obj_by_ptr};

pub fn register_game_logic_bindings(
    linker: &mut Linker<()>,
    store: &Store<()>,
    object_storage: Arc<RwLock<SlotMap<DefaultKey, Object>>>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    register_spawn_object(linker, store, object_storage.clone())?;
    register_despawn_object(linker, store, object_storage.clone())?;
    register_request_object(linker, store, object_storage.clone(), alloc_fn.clone())?;
    register_request_object_position(linker, store, object_storage.clone(), alloc_fn.clone())?;
    register_set_object_position(linker, store, object_storage.clone())?;
    Ok(())
}

pub fn register_spawn_object(
    linker: &mut Linker<()>,
    store: &Store<()>,
    object_storage: Arc<RwLock<SlotMap<DefaultKey, Object>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "spawn_object_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [wasmtime::ValType::I64].iter().cloned(),
        ),
        move |mut caller, params, results| {
            let obj = get_obj_by_ptr::<Object>(
                &mut caller,
                params[0].unwrap_i32() as u32,
                params[1].unwrap_i32() as u32,
            )?;

            let mut storage_lock = object_storage.write().unwrap();
            info!("Object spawned. obj: {:?}", &obj);
            let index = storage_lock.insert(obj);

            results[0] = wasmtime::Val::I64(KeyData::as_ffi(index.data()) as i64);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_despawn_object(
    linker: &mut Linker<()>,
    store: &Store<()>,
    object_storage: Arc<RwLock<SlotMap<DefaultKey, Object>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "despawn_object_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I64].iter().cloned(),
            [].iter().cloned(),
        ),
        move |_, params, _| {
            let index = params[0].unwrap_i64() as u64;
            let mut storage_lock = object_storage.write().unwrap();
            storage_lock.remove(KeyData::from_ffi(index).into());
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_request_object(
    linker: &mut Linker<()>,
    store: &Store<()>,
    object_storage: Arc<RwLock<SlotMap<DefaultKey, Object>>>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "request_object_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I64].iter().cloned(),
            [wasmtime::ValType::I32].iter().cloned(),
        ),
        move |mut caller, params, results| {
            let index = params[0].unwrap_i32() as u64;

            let storage_lock = object_storage.read().unwrap();
            let object: Option<&Object> = storage_lock.get(KeyData::from_ffi(index).into());
            if let Some(obj) = object {
                let obj_clone = *obj;

                copy_obj_to_memory(
                    &mut caller,
                    obj_clone,
                    alloc_fn.read().unwrap().as_ref().unwrap().clone(),
                )?;
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
    object_storage: Arc<RwLock<SlotMap<DefaultKey, Object>>>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "request_object_position_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I64].iter().cloned(),
            [wasmtime::ValType::I32].iter().cloned(),
        ),
        move |mut caller, params, results| {
            let index = params[0].unwrap_i64() as u64;

            let storage_lock = object_storage.read().unwrap();
            let object: Option<&Object> = storage_lock.get(KeyData::from_ffi(index).into());

            if let Some(obj) = object {
                let obj_clone = obj.position;

                copy_obj_to_memory(
                    &mut caller,
                    obj_clone,
                    alloc_fn.read().unwrap().as_ref().unwrap().clone(),
                )?;
            }

            results[0] = wasmtime::Val::I32(object.is_some() as i32);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_set_object_position(
    linker: &mut Linker<()>,
    store: &Store<()>,
    object_storage: Arc<RwLock<SlotMap<DefaultKey, Object>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "set_object_position_sys",
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
            let (index, ptr, len) = (
                params[0].unwrap_i64() as u64,
                params[1].unwrap_i32() as u32,
                params[2].unwrap_i32() as u32,
            );
            let mut storage_lock = object_storage.write().unwrap();
            let new_position = get_obj_by_ptr(&mut caller, ptr, len)?;
            let object: Option<&mut Object> = storage_lock.get_mut(KeyData::from_ffi(index).into());
            if let Some(obj) = object {
                obj.position = new_position;
            }
            Ok(())
        },
    )?;
    Ok(())
}
