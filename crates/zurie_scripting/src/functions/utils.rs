use crate::utils::get_string_by_ptr;
use anyhow::Ok;
use log::info;
use std::sync::{Arc, RwLock};
use wasmtime::{Caller, Linker};
use zurie_shared::DELTA_TIME;

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
