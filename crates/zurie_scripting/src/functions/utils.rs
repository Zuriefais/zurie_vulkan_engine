use crate::utils::get_string_by_ptr;
use anyhow::Ok;
use log::info;
use rand::{thread_rng, Rng};
use std::sync::{Arc, RwLock};
use wasmtime::{Caller, Linker, Store, Val};
use zurie_shared::DELTA_TIME;

pub fn register_utils_bindings(
    linker: &mut Linker<()>,
    store: &Store<()>,
    mod_name: Arc<RwLock<String>>,
) -> anyhow::Result<()> {
    register_get_rand_f32(linker, store)?;
    register_get_rand_i32(linker, store)?;
    register_get_delta_time(linker)?;
    register_info(linker, mod_name.clone())?;
    register_get_mod_name_callback(linker, mod_name.clone())?;
    Ok(())
}

pub fn register_get_delta_time(linker: &mut Linker<()>) -> anyhow::Result<()> {
    linker.func_wrap("env", "get_delta_time_sys", || -> f32 {
        unsafe { DELTA_TIME }
    })?;
    Ok(())
}

pub fn register_info(linker: &mut Linker<()>, mod_name: Arc<RwLock<String>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "env",
        "info_sys",
        move |mut caller: Caller<'_, ()>, ptr: u32, len: u32| {
            let string = get_string_by_ptr(&mut caller, ptr, len)?;
            info!(target: mod_name.read().unwrap().as_str(), "{}", string);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_get_mod_name_callback(
    linker: &mut Linker<()>,
    mod_name: Arc<RwLock<String>>,
) -> anyhow::Result<()> {
    linker.func_wrap(
        "env",
        "get_mod_name_callback",
        move |mut caller: Caller<'_, ()>, ptr: u32, len: u32| {
            let name = get_string_by_ptr(&mut caller, ptr, len)?;
            let mut data_lock = mod_name.write().unwrap();
            *data_lock = name.to_string();
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_get_rand_f32(linker: &mut Linker<()>, store: &Store<()>) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "get_rand_f32_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::F32, wasmtime::ValType::F32]
                .iter()
                .cloned(),
            [wasmtime::ValType::F32].iter().cloned(),
        ),
        move |_, params, results| {
            let (x, y) = (params[0].unwrap_f32(), params[1].unwrap_f32());
            let mut rand = thread_rng();
            let rand_num = rand.gen_range(x..y);
            results[0] = Val::F32(rand_num as u32);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn register_get_rand_i32(linker: &mut Linker<()>, store: &Store<()>) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "get_rand_i32_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [wasmtime::ValType::I32].iter().cloned(),
        ),
        move |_, params, results| {
            let (x, y) = (params[0].unwrap_i32(), params[1].unwrap_i32());
            let mut rand = thread_rng();
            let rand_num = rand.gen_range(x..y);
            results[0] = Val::I32(rand_num as i32);
            Ok(())
        },
    )?;
    Ok(())
}
