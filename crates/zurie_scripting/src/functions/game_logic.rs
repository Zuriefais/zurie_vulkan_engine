use anyhow::Ok;
use log::info;
use std::sync::{Arc, RwLock};
use wasmtime::{Linker, Store, TypedFunc};
use zurie_ecs::{ComponentID, EntityData, World};
use zurie_shared::slotmap::{Key, KeyData};
use zurie_types::{ComponentData, Object, Vector2};

use crate::utils::{copy_obj_to_memory, get_obj_by_ptr};

pub fn register_game_logic_bindings(
    linker: &mut Linker<()>,
    store: &Store<()>,
    world: Arc<RwLock<World>>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    let (pos_component, scale_component, color_component, sprite_component) = {
        let mut world = world.write().unwrap();
        (
            world.register_component("position".into()),
            world.register_component("scale".into()),
            world.register_component("color".into()),
            world.register_component("sprite".into()),
        )
    };

    register_spawn_object(
        linker,
        store,
        world.clone(),
        pos_component,
        scale_component,
        color_component,
        sprite_component,
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
    sprite_component: ComponentID,
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
                    (sprite_component, ComponentData::Sprite(obj.sprite)),
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
