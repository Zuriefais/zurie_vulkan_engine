use crate::app::DELTA_TIME;
use anyhow::Ok;
use egui_winit_vulkano::egui::{self, Context};
use hashbrown::HashSet;
use log::info;
use shared_types::{
    bitcode::{self, Decode},
    GuiTextMessage, KeyCode,
};
use std::sync::{Arc, RwLock};
use wasmtime::{Caller, Engine, Extern, Instance, Linker, Module, Store, TypedFunc};
use winit::event::WindowEvent;

pub struct ModManager {
    engine: Engine,
    gui_context: Context,
    mods: Vec<Arc<RwLock<EngineMod>>>,
    new_mod_path: String,
    pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
}

impl ModManager {
    pub fn event(&mut self, ev: WindowEvent) -> anyhow::Result<()> {
        match ev {
            WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                winit::keyboard::PhysicalKey::Code(key_code) => {
                    let key_code = key_code as u32;
                    let key_code: KeyCode = KeyCode::try_from(key_code).unwrap();
                    for engine_mod in self.mods.iter() {
                        let mut mod_lock = engine_mod.write().unwrap();
                        mod_lock.key_event(key_code.clone())?;
                    }
                }
                winit::keyboard::PhysicalKey::Unidentified(_) => {}
            },
            _ => {}
        }
        Ok(())
    }
    pub fn update(&mut self) -> anyhow::Result<()> {
        let mut reload_mods = false;
        let mut load_new_mod = false;
        egui::Window::new("Mods Window").show(&self.gui_context, |ui| {
            reload_mods = ui.button("reload mods").clicked();
            load_new_mod = ui.button("Load new mod").clicked();
            ui.label("mod path:");
            ui.text_edit_singleline(&mut self.new_mod_path);
        });
        if reload_mods {
            let mut new_mods = vec![];
            for engine_mod in self.mods.iter() {
                let mod_lock = engine_mod.read().unwrap();
                let mod_path = mod_lock.path.clone();
                new_mods.push(Arc::new(RwLock::new(EngineMod::new(
                    mod_path.clone(),
                    &self.engine,
                    self.gui_context.clone(),
                    self.pressed_keys_buffer.clone(),
                )?)));
                info!("reloading {}", mod_path);
            }
            self.mods = new_mods;
        }
        if load_new_mod {
            info!("Loading mod at path: {}", self.new_mod_path.clone());
            self.mods.push(Arc::new(RwLock::new(EngineMod::new(
                self.new_mod_path.clone(),
                &self.engine,
                self.gui_context.clone(),
                self.pressed_keys_buffer.clone(),
            )?)));
        }
        for engine_mod in self.mods.iter() {
            let mut mod_lock = engine_mod.write().unwrap();
            mod_lock.update().unwrap();
        }
        Ok(())
    }
    pub fn new(gui_context: Context, pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>) -> Self {
        let engine = Engine::default();
        let test_mod = Arc::new(RwLock::new(
            EngineMod::new(
                "./target/wasm32-unknown-unknown/release/example_mod.wasm".to_string(),
                &engine,
                gui_context.clone(),
                pressed_keys_buffer.clone(),
            )
            .expect("Error loading mod"),
        ));
        let mods = vec![test_mod];
        Self {
            engine,
            gui_context,
            mods,
            new_mod_path: String::new(),
            pressed_keys_buffer,
        }
    }
}

#[derive()]
pub struct EngineMod {
    pub path: String,
    pub module: Module,
    pub instance: Instance,
    pub store: Store<()>,
    pub update_fn: TypedFunc<(), ()>,
    pub key_event_fn: TypedFunc<u32, ()>,
    pub mod_name: Arc<RwLock<String>>,
    pub subscribed_keys: Arc<RwLock<HashSet<KeyCode>>>,
}

impl EngineMod {
    pub fn new(
        mod_path: String,
        engine: &Engine,
        gui_context: Context,
        pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
    ) -> anyhow::Result<Self> {
        let mut linker: Linker<()> = Linker::new(engine);
        let mod_name = Arc::new(RwLock::new("No name".to_string()));
        let mod_name_func = mod_name.clone();
        let mod_name_func2 = mod_name.clone();
        let mod_name_func3 = mod_name.clone();
        let module = Module::from_file(engine, &mod_path)?;
        let subscribed_keys: Arc<RwLock<HashSet<KeyCode>>> = Default::default();
        let subscribed_keys_clone = subscribed_keys.clone();
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
        let subscribe_for_key_event_sys = move |key: u32| {
            let key: KeyCode = KeyCode::try_from(key).unwrap();
            info!(target: mod_name_func3.read().unwrap().as_str(), "subscribed for {:?}", key);
            let mut keys_lock = subscribed_keys_clone.write().unwrap();
            keys_lock.insert(key);
        };
        linker.func_wrap("env", "info_sys", func_info)?;
        linker.func_wrap("env", "gui_text_sys", func_gui_text)?;
        linker.func_wrap("env", "get_mod_name_callback", func_get_mod_name_callback)?;
        linker.func_wrap(
            "env",
            "subscribe_for_key_event_sys",
            subscribe_for_key_event_sys,
        )?;
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
        linker.func_new(
            "env",
            "if_key_pressed_sys",
            wasmtime::FuncType::new(
                store.engine(),
                [wasmtime::ValType::I32].iter().cloned(),
                [wasmtime::ValType::I32].iter().cloned(),
            ),
            move |_, params, results| {
                let key: KeyCode = KeyCode::try_from(params[0].unwrap_i32() as u32).unwrap();
                let clicked = pressed_keys_buffer.read().unwrap().contains(&key) as i32;
                results[0] = wasmtime::Val::I32(clicked);
                Ok(())
            },
        )?;
        let instance = linker.instantiate(&mut store, &module)?;
        let init_fn: TypedFunc<(), ()> = instance.get_typed_func::<(), ()>(&mut store, "init")?;
        let update_fn: TypedFunc<(), ()> =
            instance.get_typed_func::<(), ()>(&mut store, "update")?;
        let key_event_fn: TypedFunc<u32, ()> =
            instance.get_typed_func::<u32, ()>(&mut store, "key_event")?;
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
            key_event_fn,
            mod_name,
            subscribed_keys,
        })
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        self.update_fn.call(&mut self.store, ())?;
        Ok(())
    }

    pub fn key_event(&mut self, key_code: KeyCode) -> anyhow::Result<()> {
        let keys_lock = self.subscribed_keys.read().unwrap();
        if keys_lock.contains(&key_code) {
            self.key_event_fn.call(&mut self.store, key_code as u32)?;
        }
        Ok(())
    }
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
    let obj = bitcode::decode(data)?;
    Ok(obj)
}
