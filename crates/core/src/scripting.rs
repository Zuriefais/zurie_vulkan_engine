use crate::app::DELTA_TIME;
use log::info;
use std::sync::{Arc, RwLock};
use wasmtime::{Caller, Engine, Extern, Instance, Linker, Module, Result, Store, TypedFunc};

pub struct EngineMod {
    pub module: Module,
    pub instance: Instance,
    pub store: Store<()>,
    pub update_fn: TypedFunc<(), ()>,
    pub mod_name: Arc<RwLock<String>>,
}

impl EngineMod {
    pub fn new(mod_path: String, engine: &Engine) -> Result<Self, wasmtime::Error> {
        let mut linker: Linker<()> = Linker::new(engine);
        let mod_name = Arc::new(RwLock::new("No name".to_string()));
        let mod_name_func = mod_name.clone();
        let mod_name_func2 = mod_name.clone();
        //preview1::add_to_linker_sync(&mut linker, |t| t)?;
        let module = Module::from_file(engine, &mod_path)?;
        info!("mod at path {} compiled", mod_path);

        //let wasi = WasiCtxBuilder::new().inherit_stdio().build_p1();
        let mut store = Store::new(engine, ());
        linker.func_wrap("env", "get_delta_time_sys", || -> f32 {
            unsafe { DELTA_TIME }
        })?;
        let func_info = move |caller: Caller<'_, ()>, ptr: u32, len: u32| {
            let string = get_string_by_ptr(caller, ptr, len)?;
            info!(target: mod_name_func2.read().unwrap().as_str(), "{}", string);
            Ok(())
        };
        let func_get_mod_name_callback = move |caller: Caller<'_, ()>, ptr: u32, len: u32| {
            let name = get_string_by_ptr(caller, ptr, len)?;
            let mut data_lock = mod_name_func.write().unwrap();
            *data_lock = name.to_string();
            Ok(())
        };
        linker.func_wrap("env", "info_sys", func_info)?;
        linker.func_wrap("env", "get_mod_name_callback", func_get_mod_name_callback)?;
        let instance = linker.instantiate(&mut store, &module)?;
        let init_fn: TypedFunc<(), ()> = instance.get_typed_func::<(), ()>(&mut store, "init")?;
        let update_fn: TypedFunc<(), ()> =
            instance.get_typed_func::<(), ()>(&mut store, "update")?;
        let get_mod_name_fn: TypedFunc<(), ()> =
            instance.get_typed_func::<(), ()>(&mut store, "get_mod_name")?;
        get_mod_name_fn.call(&mut store, ())?;
        info!("Mod name: {}", mod_name.read().unwrap());
        init_fn.call(&mut store, ())?;
        Ok(Self {
            module,
            instance,
            store,
            update_fn,
            mod_name,
        })
    }

    pub fn update(&mut self) -> Result<(), wasmtime::Error> {
        self.update_fn.call(&mut self.store, ())?;
        Ok(())
    }
}

fn get_string_by_ptr(
    mut caller: Caller<'_, ()>,
    ptr: u32,
    len: u32,
) -> Result<String, wasmtime::Error> {
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => anyhow::bail!("failed to find host memory"),
    };
    let data = mem
        .data(&caller)
        .get(ptr as usize..)
        .and_then(|arr| arr.get(..len as usize));
    let str = match data {
        Some(data) => match std::str::from_utf8(data) {
            Ok(s) => s,
            Err(_) => anyhow::bail!("invalid utf-8"),
        },
        None => anyhow::bail!("pointer/length out of bounds"),
    };
    Ok(str.to_string())
}
