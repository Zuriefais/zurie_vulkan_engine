// fn request_camera_sys();
// fn set_camera_sys(ptr: u32, len: u32);
// fn set_zoom_factor_sys(factor: f32);
// fn get_zoom_factor_sys() -> f32;

use std::sync::{Arc, RwLock};

use log::info;
use wasmtime::{Linker, Store};
use zurie_types::camera::Camera;

use crate::utils::{copy_obj_to_memory, get_obj_by_ptr};

pub fn register_camera_bindings(
    linker: &mut Linker<()>,
    camera: Arc<RwLock<Camera>>,
    store: &Store<()>,
) -> anyhow::Result<()> {
    register_request_camera(linker, camera.clone(), store)?;
    register_set_camera(linker, camera.clone(), store)?;
    register_set_zoom_factor(linker, camera.clone(), store)?;
    register_get_zoom_factor(linker, camera.clone(), store)?;
    register_request_object_position(linker, camera.clone(), store)?;
    register_set_object_position(linker, camera.clone(), store)?;
    Ok(())
}

fn register_request_camera(
    linker: &mut Linker<()>,
    camera: Arc<RwLock<Camera>>,
    store: &Store<()>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "request_camera_sys",
        wasmtime::FuncType::new(store.engine(), [].iter().cloned(), [].iter().cloned()),
        move |mut caller, _, _| {
            let camera = camera.read().unwrap();

            let alloc_fn = caller
                .get_export("alloc")
                .and_then(|export| export.into_func())
                .ok_or_else(|| anyhow::anyhow!("Failed to find 'alloc' function"))?
                .typed::<u32, u32>(&caller)?;
            copy_obj_to_memory(&mut caller, *camera, alloc_fn.clone())?;

            Ok(())
        },
    )?;
    Ok(())
}
fn register_set_camera(
    linker: &mut Linker<()>,
    camera: Arc<RwLock<Camera>>,
    store: &Store<()>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "set_camera_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [].iter().cloned(),
        ),
        move |mut caller, params, _| {
            let (ptr, len) = (params[0].unwrap_i32() as u32, params[1].unwrap_i32() as u32);
            let new_camera: Camera = get_obj_by_ptr(&mut caller, ptr, len)?;
            *camera.write().unwrap() = new_camera;
            Ok(())
        },
    )?;
    Ok(())
}

fn register_set_zoom_factor(
    linker: &mut Linker<()>,
    camera: Arc<RwLock<Camera>>,
    store: &Store<()>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "set_zoom_factor_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::F32].iter().cloned(),
            [].iter().cloned(),
        ),
        move |_, params, _| {
            let zoom_factor = params[0].unwrap_f32();
            camera.write().unwrap().zoom_factor = zoom_factor;
            Ok(())
        },
    )?;
    Ok(())
}
fn register_get_zoom_factor(
    linker: &mut Linker<()>,
    camera: Arc<RwLock<Camera>>,
    store: &Store<()>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "get_zoom_factor_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [].iter().cloned(),
            [wasmtime::ValType::F32].iter().cloned(),
        ),
        move |_, _, results| {
            results[0] = wasmtime::Val::F32(camera.read().unwrap().zoom_factor.to_bits());
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_request_object_position(
    linker: &mut Linker<()>,
    camera: Arc<RwLock<Camera>>,
    store: &Store<()>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "request_camera_position_sys",
        wasmtime::FuncType::new(store.engine(), [].iter().cloned(), [].iter().cloned()),
        move |mut caller, _, _| {
            let camera = camera.read().unwrap();
            let alloc_fn = caller
                .get_export("alloc")
                .and_then(|export| export.into_func())
                .ok_or_else(|| anyhow::anyhow!("Failed to find 'alloc' function"))?
                .typed::<u32, u32>(&caller)?;
            copy_obj_to_memory(&mut caller, camera.position, alloc_fn.clone())?;

            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_set_object_position(
    linker: &mut Linker<()>,
    camera: Arc<RwLock<Camera>>,
    store: &Store<()>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "set_camera_position_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [].iter().cloned(),
        ),
        move |mut caller, params, _| {
            let (ptr, len) = (params[0].unwrap_i32() as u32, params[1].unwrap_i32() as u32);
            let mut camera = camera.write().unwrap();
            let new_position = get_obj_by_ptr(&mut caller, ptr, len)?;
            info!("new camera position: {}", new_position);
            camera.position = new_position;
            Ok(())
        },
    )?;
    Ok(())
}
