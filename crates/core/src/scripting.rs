use std::{env::Args, rc::Rc, sync::Arc};

use log::info;
use wasmtime::{Engine, Func, Instance, Linker, Module, Result, Store, TypedFunc, Val};
use wasmtime_wasi::{
    preview1::{self, WasiP1Ctx},
    WasiCtxBuilder,
};

use crate::app::DELTA_TIME;

pub struct EngineMod {
    pub module: Module,
    pub instance: Instance,
    pub store: Store<WasiP1Ctx>,
    pub update_fn: TypedFunc<(), ()>,
}

impl EngineMod {
    pub fn new(mod_path: String, engine: &Engine) -> Result<Self, wasmtime::Error> {
        let mut linker: Linker<WasiP1Ctx> = Linker::new(engine);
        preview1::add_to_linker_sync(&mut linker, |t| t)?;
        let module = Module::from_file(engine, &mod_path)?;
        info!("mod at path {} compiled", mod_path);

        let wasi = WasiCtxBuilder::new().inherit_stdio().build_p1();
        let mut store = Store::new(engine, wasi);
        linker.func_wrap("env", "get_delta_time_sys", || -> f32 {
            unsafe { DELTA_TIME }
        })?;
        let instance = linker.instantiate(&mut store, &module)?;
        let init_fn: TypedFunc<(), ()> = instance.get_typed_func::<(), ()>(&mut store, "init")?;
        let update_fn: TypedFunc<(), ()> =
            instance.get_typed_func::<(), ()>(&mut store, "update")?;
        init_fn.call(&mut store, ())?;
        Ok(Self {
            module,
            instance,
            store,
            update_fn,
        })
    }

    pub fn update(&mut self) -> Result<(), wasmtime::Error> {
        self.update_fn.call(&mut self.store, ())?;
        Ok(())
    }
}
