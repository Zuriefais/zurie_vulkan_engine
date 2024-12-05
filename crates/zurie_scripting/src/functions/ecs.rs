use anyhow::Ok;
use std::sync::{Arc, RwLock};
use wasmtime::{Func, Linker, Store, TypedFunc};
use zurie_ecs::{Architype, ComponentID, World};
use zurie_shared::slotmap::{Key, KeyData};
use zurie_types::{ComponentData, Query, Vector2};

use crate::utils::{copy_obj_to_memory, copy_string_to_memory, get_obj_by_ptr, get_string_by_ptr};

pub fn register_ecs_bindings(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    let sprite_component = world.write().unwrap().register_component("sprite".into());
    register_spawn_entity(linker, store, world.clone())?;
    register_despawn_entity(linker, store, world.clone())?;
    register_register_component(linker, store, world.clone())?;
    register_set_component_raw(linker, store, world.clone())?;
    register_set_component_string(linker, store, world.clone())?;
    register_set_component_color(linker, store, world.clone())?;
    register_set_component_vec2(linker, store, world.clone())?;
    register_set_component_none(linker, store, world.clone())?;
    register_set_component_sprite(linker, store, world.clone(), sprite_component)?;
    register_get_component_raw(linker, store, world.clone(), alloc_fn.clone())?;
    register_get_component_obj(linker, store, world.clone(), alloc_fn.clone())?;
    register_get_component_string(linker, store, world.clone(), alloc_fn.clone())?;
    register_get_entities_with_architype(linker, store, world.clone(), alloc_fn.clone())?;
    register_register_query(linker, store, world, alloc_fn)?;
    Ok(())
}

fn register_spawn_entity(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "spawn_entity_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [].iter().cloned(),
            [wasmtime::ValType::I64].iter().cloned(),
        ),
        move |_, _, results| {
            let mut world_lock = world.write().unwrap();
            let entity = world_lock.spawn_entity();
            results[0] = wasmtime::Val::I64(KeyData::as_ffi(entity.data()) as i64);
            Ok(())
        },
    )?;
    Ok(())
}

fn register_despawn_entity(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "despawn_entity_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I64].iter().cloned(),
            [].iter().cloned(),
        ),
        move |_, params, _| {
            let entity_id = KeyData::from_ffi(params[0].unwrap_i64() as u64);
            let mut world_lock = world.write().unwrap();
            world_lock.despawn(entity_id.into());
            Ok(())
        },
    )?;
    Ok(())
}

fn register_register_component(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "register_component_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [wasmtime::ValType::I64].iter().cloned(),
        ),
        move |mut caller, params, results| {
            let (ptr, len) = (params[0].unwrap_i32() as u32, params[1].unwrap_i32() as u32);

            // Get component name from WASM memory
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut buffer = vec![0u8; len as usize];
            memory.read(&caller, ptr as usize, &mut buffer)?;
            let component_name = String::from_utf8(buffer).unwrap();

            let mut world_lock = world.write().unwrap();
            let component_id = world_lock.register_component(component_name);
            results[0] = wasmtime::Val::I64(KeyData::as_ffi(component_id.data()) as i64);
            Ok(())
        },
    )?;
    Ok(())
}

fn register_set_component_raw(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "set_component_raw_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [
                wasmtime::ValType::I64, // entity id
                wasmtime::ValType::I64, // component id
                wasmtime::ValType::I32, // data ptr
                wasmtime::ValType::I32, // data len
            ]
            .iter()
            .cloned(),
            [].iter().cloned(),
        ),
        move |mut caller, params, _| {
            let (entity_id, component_id, data_ptr, data_len) = (
                KeyData::from_ffi(params[0].unwrap_i64() as u64),
                KeyData::from_ffi(params[1].unwrap_i64() as u64),
                params[2].unwrap_i32() as u32,
                params[3].unwrap_i32() as u32,
            );

            // Get component data from WASM memory
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut buffer = vec![0u8; data_len as usize];
            memory.read(&caller, data_ptr as usize, &mut buffer)?;

            let mut world_lock = world.write().unwrap();
            world_lock.set_component(
                entity_id.into(),
                (component_id.into(), ComponentData::Raw(buffer)),
            );
            Ok(())
        },
    )?;
    Ok(())
}

fn register_set_component_none(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "set_component_none_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [
                wasmtime::ValType::I64, // entity id
                wasmtime::ValType::I64, // component id
            ]
            .iter()
            .cloned(),
            [].iter().cloned(),
        ),
        move |_, params, _| {
            let (entity_id, component_id) = (
                KeyData::from_ffi(params[0].unwrap_i64() as u64),
                KeyData::from_ffi(params[1].unwrap_i64() as u64),
            );

            let mut world_lock = world.write().unwrap();
            world_lock.set_component(entity_id.into(), (component_id.into(), ComponentData::None));
            Ok(())
        },
    )?;
    Ok(())
}

fn register_set_component_sprite(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
    sprite_id: ComponentID,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "set_component_sprite_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [
                wasmtime::ValType::I64, // entity id
                wasmtime::ValType::I64, // sprite handle
            ]
            .iter()
            .cloned(),
            [].iter().cloned(),
        ),
        move |_, params, _| {
            let (entity_id, handle) = (
                KeyData::from_ffi(params[0].unwrap_i64() as u64),
                params[0].unwrap_i64() as u64,
            );

            let mut world_lock = world.write().unwrap();
            world_lock.set_component(entity_id.into(), (sprite_id, ComponentData::Sprite(handle)));
            Ok(())
        },
    )?;
    Ok(())
}

fn register_set_component_string(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "set_component_string_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [
                wasmtime::ValType::I64, // entity id
                wasmtime::ValType::I64, // component id
                wasmtime::ValType::I32, // data ptr
                wasmtime::ValType::I32, // data len
            ]
            .iter()
            .cloned(),
            [].iter().cloned(),
        ),
        move |mut caller, params, _| {
            let (entity_id, component_id, data_ptr, data_len) = (
                KeyData::from_ffi(params[0].unwrap_i64() as u64),
                KeyData::from_ffi(params[1].unwrap_i64() as u64),
                params[2].unwrap_i32() as u32,
                params[3].unwrap_i32() as u32,
            );

            let string = get_string_by_ptr(&mut caller, data_ptr, data_len)?;
            let mut world_lock = world.write().unwrap();
            world_lock.set_component(
                entity_id.into(),
                (component_id.into(), ComponentData::String(string)),
            );
            Ok(())
        },
    )?;
    Ok(())
}

fn register_set_component_vec2(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "set_component_vec_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [
                wasmtime::ValType::I64, // entity id
                wasmtime::ValType::I64, // component id
                wasmtime::ValType::I32, // data ptr
                wasmtime::ValType::I32, // data len
            ]
            .iter()
            .cloned(),
            [].iter().cloned(),
        ),
        move |mut caller, params, _| {
            let (entity_id, component_id, data_ptr, data_len) = (
                KeyData::from_ffi(params[0].unwrap_i64() as u64),
                KeyData::from_ffi(params[1].unwrap_i64() as u64),
                params[2].unwrap_i32() as u32,
                params[3].unwrap_i32() as u32,
            );

            // Get component data from WASM memory
            let vec2: Vector2 = get_obj_by_ptr(&mut caller, data_ptr, data_len)?;

            let mut world_lock = world.write().unwrap();
            world_lock.set_component(
                entity_id.into(),
                (component_id.into(), ComponentData::Vector(vec2)),
            );
            Ok(())
        },
    )?;
    Ok(())
}

fn register_set_component_color(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "set_component_color_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [
                wasmtime::ValType::I64, // entity id
                wasmtime::ValType::I64, // component id
                wasmtime::ValType::I32, // data ptr
                wasmtime::ValType::I32, // data len
            ]
            .iter()
            .cloned(),
            [].iter().cloned(),
        ),
        move |mut caller, params, _| {
            let (entity_id, component_id, data_ptr, data_len) = (
                KeyData::from_ffi(params[0].unwrap_i64() as u64),
                KeyData::from_ffi(params[1].unwrap_i64() as u64),
                params[2].unwrap_i32() as u32,
                params[3].unwrap_i32() as u32,
            );

            // Get component data from WASM memory
            let color: [f32; 4] = get_obj_by_ptr(&mut caller, data_ptr, data_len)?;

            let mut world_lock = world.write().unwrap();
            world_lock.set_component(
                entity_id.into(),
                (component_id.into(), ComponentData::Color(color)),
            );
            Ok(())
        },
    )?;
    Ok(())
}

fn register_get_component_raw(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "get_component_raw_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [
                wasmtime::ValType::I64, // entity id
                wasmtime::ValType::I64, // component id
            ]
            .iter()
            .cloned(),
            [wasmtime::ValType::I32].iter().cloned(), // success flag
        ),
        move |mut caller, params, results| {
            let (entity_id, component_id) = (
                KeyData::from_ffi(params[0].unwrap_i64() as u64),
                KeyData::from_ffi(params[1].unwrap_i64() as u64),
            );

            let world_lock = world.read().unwrap();
            let component = world_lock.get_component(entity_id.into(), component_id.into());

            if let Some(ComponentData::Raw(data)) = component {
                let alloc = alloc_fn.read().unwrap().as_ref().unwrap().clone();
                let ptr = alloc.call(&mut caller, data.len() as u32)?;

                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                memory.write(&mut caller, ptr as usize, data)?;

                results[0] = wasmtime::Val::I32(1); // Success
            } else {
                results[0] = wasmtime::Val::I32(0); // Failure
            }

            Ok(())
        },
    )?;
    Ok(())
}

fn register_get_component_obj(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "get_component_obj_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [
                wasmtime::ValType::I64, // entity id
                wasmtime::ValType::I64, // component id
            ]
            .iter()
            .cloned(),
            [wasmtime::ValType::I32].iter().cloned(), // success flag
        ),
        move |mut caller, params, results| {
            let (entity_id, component_id) = (
                KeyData::from_ffi(params[0].unwrap_i64() as u64),
                KeyData::from_ffi(params[1].unwrap_i64() as u64),
            );

            let world_lock = world.read().unwrap();
            let component = world_lock.get_component(entity_id.into(), component_id.into());

            if let Some(component) = component {
                match *component {
                    ComponentData::Vector(vec) => {
                        copy_obj_to_memory(
                            &mut caller,
                            vec,
                            alloc_fn.read().unwrap().as_ref().unwrap().clone(),
                        )?;
                        results[0] = wasmtime::Val::I32(1);
                        return Ok(());
                    }
                    ComponentData::Color(color) => {
                        copy_obj_to_memory(
                            &mut caller,
                            color,
                            alloc_fn.read().unwrap().as_ref().unwrap().clone(),
                        )?;
                        results[0] = wasmtime::Val::I32(1);
                        return Ok(());
                    }

                    _ => {}
                }
            }
            results[0] = wasmtime::Val::I32(0);
            Ok(())
        },
    )?;
    Ok(())
}

fn register_get_component_string(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "get_component_string_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [
                wasmtime::ValType::I64, // entity id
                wasmtime::ValType::I64, // component id
            ]
            .iter()
            .cloned(),
            [wasmtime::ValType::I32].iter().cloned(), // success flag
        ),
        move |mut caller, params, results| {
            let (entity_id, component_id) = (
                KeyData::from_ffi(params[0].unwrap_i64() as u64),
                KeyData::from_ffi(params[1].unwrap_i64() as u64),
            );

            let world_lock = world.read().unwrap();
            let component = world_lock.get_component(entity_id.into(), component_id.into());

            if let Some(ComponentData::String(string)) = component {
                copy_string_to_memory(
                    &mut caller,
                    string,
                    alloc_fn.read().unwrap().as_ref().unwrap().clone(),
                )?;
                results[0] = wasmtime::Val::I32(1);
                return Ok(());
            }
            results[0] = wasmtime::Val::I32(0);
            Ok(())
        },
    )?;
    Ok(())
}

fn register_get_entities_with_architype(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "get_entities_with_architype_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [].iter().cloned(),
        ),
        move |mut caller, params, _| {
            let (ptr, len) = (params[0].unwrap_i32() as u32, params[1].unwrap_i32() as u32);

            // Get the architype components from WASM memory
            let architype: Vec<u64> = get_obj_by_ptr(&mut caller, ptr, len)?;

            // Convert the raw component IDs to ComponentID
            let components: Vec<ComponentID> = architype
                .into_iter()
                .map(|id| KeyData::from_ffi(id).into())
                .collect();

            let entities = {
                let world = world.read().unwrap();
                world
                    .get_entities_with_arhetype(Architype { data: components })
                    .iter()
                    .map(|entity| entity.data().as_ffi())
                    .collect::<Vec<u64>>()
            };

            // Copy the result back to WASM memory
            let alloc = alloc_fn.read().unwrap().as_ref().unwrap().clone();
            copy_obj_to_memory(&mut caller, entities, alloc)?;

            Ok(())
        },
    )?;
    Ok(())
}

fn register_register_query(
    linker: &mut Linker<()>,
    store: &Store<()>,
    _world: Arc<RwLock<World>>,
    _alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "register_query_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [].iter().cloned(),
        ),
        move |mut caller, params, _| {
            let (ptr, len) = (params[0].unwrap_i32() as u32, params[1].unwrap_i32() as u32);

            let query: Query = get_obj_by_ptr(&mut caller, ptr, len)?;
            let func: Func = caller
                .get_export(&query.name)
                .and_then(|e| e.into_func())
                .unwrap();
            let func = func.typed::<(), ()>(&caller)?;
            func.call(caller, ())?;
            Ok(())
        },
    )?;
    Ok(())
}
