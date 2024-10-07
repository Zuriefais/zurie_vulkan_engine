use crate::app::DELTA_TIME;
use anyhow::Ok;
use egui_winit_vulkano::egui::{self, Context, Ui};
use log::info;
use shared_types::{
    bitcode::{self, Decode},
    GuiTextMessage,
};
use std::sync::{Arc, RwLock};
use wasmtime::{Caller, Engine, Extern, Instance, Linker, Module, Result, Store, TypedFunc};

#[derive()]
pub struct EngineMod {
    pub path: String,
    pub module: Module,
    pub instance: Instance,
    pub store: Store<()>,
    pub update_fn: TypedFunc<(), ()>,
    pub mod_name: Arc<RwLock<String>>,
    gui_context: Context,
}

impl EngineMod {
    pub fn new(
        mod_path: String,
        engine: &Engine,
        gui_context: Context,
    ) -> Result<Self, wasmtime::Error> {
        let mut linker: Linker<()> = Linker::new(engine);
        let mod_name = Arc::new(RwLock::new("No name".to_string()));
        let mod_name_func = mod_name.clone();
        let mod_name_func2 = mod_name.clone();
        let module = Module::from_file(engine, &mod_path)?;
        info!("mod at path {} compiled", mod_path);
        let mut store = Store::new(engine, ());
        linker.func_wrap("env", "get_delta_time_sys", || -> f32 {
            unsafe { DELTA_TIME }
        })?;
        let func_info = move |caller: Caller<'_, ()>, ptr: u32, len: u32| {
            let string = get_string_by_ptr(caller, ptr, len)?;
            info!(target: mod_name_func2.read().unwrap().as_str(), "{}", string);
            Ok(())
        };
        let gui_context_clone = gui_context.clone();
        let gui_context_clone2 = gui_context.clone();
        let func_gui_text = move |caller: Caller<'_, ()>, ptr: u32, len: u32| {
            let obj = get_obj_by_ptr::<GuiTextMessage>(caller, ptr, len).unwrap();
            let window = egui::Window::new(obj.window_title);
            window.show(&gui_context_clone, |ui| ui.label(obj.label_text));
        };
        let func_get_mod_name_callback = move |caller: Caller<'_, ()>, ptr: u32, len: u32| {
            let name = get_string_by_ptr(caller, ptr, len)?;
            let mut data_lock = mod_name_func.write().unwrap();
            *data_lock = name.to_string();
            Ok(())
        };
        linker.func_wrap("host", "double", |x: i32| x * 2)?;
        linker.func_wrap("env", "info_sys", func_info)?;
        linker.func_wrap("env", "gui_text_sys", func_gui_text)?;
        linker.func_wrap("env", "get_mod_name_callback", func_get_mod_name_callback)?;
        linker.func_new(
            "env",
            "gui_button_sys",
            wasmtime::FuncType::new(
                store.engine(),
                [wasmtime::ValType::I32, wasmtime::ValType::I32]
                    .iter()
                    .cloned(),
                [wasmtime::ValType::I32].iter().cloned(),
            ),
            move |caller, params, results| {
                let obj = get_obj_by_ptr::<GuiTextMessage>(
                    caller,
                    params[0].unwrap_i32() as u32,
                    params[1].unwrap_i32() as u32,
                )?;
                let mut clicked = 0;
                let window = egui::Window::new(obj.window_title);
                window.show(&gui_context_clone2, |ui| {
                    clicked = ui.button(obj.label_text).clicked() as i32;
                });
                results[0] = wasmtime::Val::I32(clicked);
                Ok(())
            },
        )?;
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
            path: mod_path,
            module,
            instance,
            store,
            update_fn,
            mod_name,
            gui_context,
        })
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        self.update_fn.call(&mut self.store, ())?;
        Ok(())
    }

    pub fn event() {}
}

fn get_string_by_ptr(mut caller: Caller<'_, ()>, ptr: u32, len: u32) -> anyhow::Result<String> {
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => anyhow::bail!("failed to find host memory"),
    };
    let data = mem
        .data(&caller)
        .get(ptr as usize..)
        .and_then(|arr| arr.get(..len as usize))
        .unwrap();
    Ok(std::str::from_utf8(data)?.to_string())
}

fn get_obj_by_ptr<T: for<'a> Decode<'a>>(
    mut caller: Caller<'_, ()>,
    ptr: u32,
    len: u32,
) -> anyhow::Result<T> {
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => anyhow::bail!("failed to find host memory"),
    };
    let data = mem
        .data(&caller)
        .get(ptr as usize..)
        .and_then(|arr| arr.get(..len as usize))
        .unwrap();
    let obj = bitcode::decode(&data)?;
    Ok(obj)
}
