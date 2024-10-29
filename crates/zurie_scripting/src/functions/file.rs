use crate::utils::{copy_to_memory, get_string_by_ptr};
use anyhow::Ok;
use log::{error, info};
use std::sync::Arc;
use std::{fs, sync::RwLock};
use wasmtime::{Linker, Store, TypedFunc, Val};

pub fn register_file_bindings(
    linker: &mut Linker<()>,
    store: &Store<()>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    register_load_file(linker, store, alloc_fn)?;

    Ok(())
}

pub fn register_load_file(
    linker: &mut Linker<()>,
    store: &Store<()>,
    alloc_fn: Arc<RwLock<Option<TypedFunc<u32, u32>>>>,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "load_file_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [wasmtime::ValType::I32].iter().cloned(),
        ),
        move |mut caller, params, results| -> anyhow::Result<()> {
            let path = get_string_by_ptr(
                &mut caller,
                params[0].unwrap_i32() as u32,
                params[1].unwrap_i32() as u32,
            )?;
            match fs::read(&path) {
                std::result::Result::Ok(data) => {
                    info!("File loaded at path: {}", path);
                    copy_to_memory(
                        &mut caller,
                        &data,
                        alloc_fn.read().unwrap().as_ref().unwrap().clone(),
                    )?;
                    results[0] = Val::I32(1)
                }
                Err(_) => {
                    error!("Error loading file at path: {}", path);
                    results[0] = Val::I32(0)
                }
            }

            Ok(())
        },
    )?;
    Ok(())
}
