use anyhow::Ok;
use log::info;
use std::sync::{Arc, RwLock};
use wasmtime::{Linker, Store, TypedFunc};
use zurie_ecs::{ComponentID, EntityData, World};
use zurie_shared::slotmap::{Key, KeyData};
use zurie_types::{ComponentData, Object, Vector2};

use crate::utils::{copy_obj_to_memory, copy_string_to_memory, get_obj_by_ptr, get_string_by_ptr};

pub fn register_game_logic_bindings(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    let (pos_component, scale_component, color_component) = {
        let mut world = world.write().unwrap();
        (
            world.register_component("position".into()),
            world.register_component("scale".into()),
            world.register_component("color".into()),
        )
    };

    register_spawn_object(
        linker,
        store,
        world.clone(),
        pos_component,
        scale_component,
        color_component,
    )?;
    register_despawn_object(linker, store, world.clone())?;
    register_request_object(linker, store, world.clone(), alloc_fn.clone())?;
    register_request_object_position(
        linker,
        store,
        world.clone(),
        alloc_fn.clone(),
        pos_component,
    )?;
    register_set_object_position(linker, store, world.clone(), pos_component)?;
    Ok(())
}

pub fn register_spawn_object(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
    pos_component: ComponentID,
    scale_component: ComponentID,
    color_component: ComponentID,
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

            let mut world_lock = world.write().unwrap();
            info!("Object spawned. obj: {:?}", &obj);
            let ent_data = EntityData {
                data: vec![
                    (pos_component, obj.position.into()),
                    (scale_component, obj.scale.into()),
                    (color_component, obj.color.into()),
                ],
            };
            let entity = world_lock.spawn_entity_with_data(ent_data);

            results[0] = wasmtime::Val::I64(KeyData::as_ffi(entity.data()) as i64);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_despawn_object(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
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
            let mut world_lock = world.write().unwrap();
            world_lock.despawn(KeyData::from_ffi(index).into());
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_request_object(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
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
            let entity = KeyData::from_ffi(params[0].unwrap_i64() as u64);

            let world_lock = world.read().unwrap();
            //let object: Option<&Object> = storage_lock.get();
            let object = world_lock.get_entity_data(entity.into());
            if let Some(object) = object {
                copy_obj_to_memory(
                    &mut caller,
                    object.data.clone(),
                    alloc_fn.read().unwrap().as_ref().unwrap().clone(),
                )?;
            }

            results[0] = wasmtime::Val::I32(object.is_some() as i32);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_request_object_position(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
    position_component: ComponentID,
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
            let entity = KeyData::from_ffi(params[0].unwrap_i64() as u64);

            let world_lock = world.read().unwrap();
            //let object: Option<&Object> = storage_lock.get();
            let pos_component = world_lock.get_component(entity.into(), position_component);
            if let Some(pos) = pos_component {
                if let ComponentData::Vector(vector2) = pos {
                    copy_obj_to_memory(
                        &mut caller,
                        vector2,
                        alloc_fn.read().unwrap().as_ref().unwrap().clone(),
                    )?
                }
            }

            results[0] = wasmtime::Val::I32(pos_component.is_some() as i32);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_set_object_position(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
    position_component: ComponentID,
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
            let (entity, ptr, len) = (
                KeyData::from_ffi(params[0].unwrap_i64() as u64),
                params[1].unwrap_i32() as u32,
                params[2].unwrap_i32() as u32,
            );
            let mut world_lock = world.write().unwrap();
            let new_position: Vector2 = get_obj_by_ptr(&mut caller, ptr, len)?;
            //let object: Option<&mut Object> = storage_lock.get_mut(KeyData::from_ffi(index).into());
            // if let Some(obj) = object {
            //     obj.position = new_position;
            // }
            world_lock.set_component(entity.into(), (position_component, new_position.into()));
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_ecs_bindings(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    register_spawn_entity(linker, store, world.clone())?;
    register_despawn_entity(linker, store, world.clone())?;
    register_register_component(linker, store, world.clone())?;
    register_set_component_raw(linker, store, world.clone())?;
    register_set_component_string(linker, store, world.clone())?;
    register_set_component_color(linker, store, world.clone())?;
    register_set_component_scale(linker, store, world.clone())?;
    register_set_component_vec2(linker, store, world.clone())?;
    register_get_component_raw(linker, store, world.clone(), alloc_fn.clone())?;
    register_get_component_obj(linker, store, world.clone(), alloc_fn.clone())?;
    register_get_component_string(linker, store, world.clone(), alloc_fn.clone())?;
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

fn register_set_component_scale(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "set_component_scale_sys",
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
            let scale: [f32; 2] = get_obj_by_ptr(&mut caller, data_ptr, data_len)?;

            let mut world_lock = world.write().unwrap();
            world_lock.set_component(
                entity_id.into(),
                (component_id.into(), ComponentData::Scale(scale)),
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
                    ComponentData::Scale(scale) => {
                        copy_obj_to_memory(
                            &mut caller,
                            scale,
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
                    &string,
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
