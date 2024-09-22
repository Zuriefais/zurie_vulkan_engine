use log::info;
use std::fs::File;
use wasmtime::{Caller, Engine, Func, Instance, Linker, Module, Result, Store};
use wasmtime_wasi::{
    preview1::{self, WasiP1Ctx},
    WasiCtxBuilder,
};

pub struct EngineMod {
    pub module: Module,
    pub instance: Instance,
}

impl EngineMod {
    pub fn new(mod_path: String, engine: &Engine) -> Result<Self, wasmtime::Error> {
        let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
        preview1::add_to_linker_sync(&mut linker, |t| t);
        let module = Module::from_file(engine, &mod_path)?;
        info!("mod at path {} compiled", mod_path);
        let wasi = WasiCtxBuilder::new().inherit_stdio().build_p1();
        let mut store = Store::new(&engine, wasi);
        let instance = linker.instantiate(&mut store, &module)?;

        let init_fn = instance.get_typed_func::<(), ()>(&mut store, "init")?;
        init_fn.call(store, ())?;
        Ok(Self { module, instance })
    }
}
