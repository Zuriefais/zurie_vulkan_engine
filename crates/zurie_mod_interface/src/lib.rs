pub mod input;

use crate::input::KeyCode;

pub use log;

pub use once_cell;
use zurie::engine::core::{Vec2, debug, error, info, trace, warn};
wit_bindgen::generate!({
    path: "../zurie_scripting/zurie_engine.wit",
    world: "zurie-mod",
});

pub use zurie::engine;

use std::ops::DerefMut;
use std::sync::{Mutex, OnceLock};

pub struct EngineLogger;

impl log::Log for EngineLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let message = record.args().to_string();
            let module_path = record
                .module_path()
                .unwrap_or_else(|| "zurie_mod_interface");

            match record.level() {
                log::Level::Error => error(module_path, &message),
                log::Level::Warn => warn(module_path, &message),
                log::Level::Info => info(module_path, &message),
                log::Level::Debug => debug(module_path, &message),
                log::Level::Trace => trace(module_path, &message),
            }
        }
    }

    fn flush(&self) {}
}

pub struct APIWraper;
use log::warn;
use log::{Level, LevelFilter};

impl Guest for APIWraper {
    fn init() {
        log::set_boxed_logger(Box::new(EngineLogger))
            .map(|()| log::set_max_level(LevelFilter::Info))
            .unwrap();

        zurie_mod().init();
    }

    fn update() {
        zurie_mod().update();
    }

    fn key_event(key_code: u32) {
        let key_code = KeyCode::try_from(key_code).unwrap();
        zurie_mod().key_event(key_code);
    }

    fn scroll(amount: f32) {
        zurie_mod().scroll(amount);
    }

    fn event(handle: EventHandle, data: EventData) {
        zurie_mod().event(handle, data);
    }
}
fn zurie_mod() -> impl DerefMut<Target = Box<dyn ZurieMod>> {
    ZURIE_MOD.get().unwrap().lock().unwrap()
}
export!(APIWraper);

pub static ZURIE_MOD: OnceLock<Mutex<Box<dyn ZurieMod>>> = OnceLock::new();

#[macro_export]
macro_rules! register_zurie_mod {
    ($mod_type:ty) => {
        #[used]
        #[unsafe(link_section = ".init_array")]
        static ZURIE_MOD_INIT: fn() = {
            fn init() {
                use std::sync::Mutex;
                use $crate::{ZURIE_MOD, ZurieMod};

                let instance = <$mod_type as ZurieMod>::new();
                let boxed = Box::new(instance) as Box<dyn ZurieMod>;

                let _ = ZURIE_MOD.set(Mutex::new(boxed));
            }
            init
        };
    };
}

pub trait ZurieMod: Send + Sync {
    fn update(&mut self) {
        warn!("Update is't implamented")
    }
    fn key_event(&mut self, key: KeyCode) {
        warn!("Key event handler is't implamented")
    }
    fn scroll(&mut self, amount: f32) {
        warn!("Scroll event handler is't implamented")
    }
    fn new() -> Self
    where
        Self: Sized;
    fn init(&mut self) {
        warn!("Init is't implamented")
    }
    fn event(&mut self, handle: EventHandle, data: EventData) {
        warn!("Generic event handler is't implamented")
    }
    fn get_mod_name(&self) -> String;
}

impl From<glam::Vec2> for Vec2 {
    fn from(vec: glam::Vec2) -> Self {
        Self { x: vec.x, y: vec.y }
    }
}

impl Into<glam::Vec2> for Vec2 {
    fn into(self) -> glam::Vec2 {
        glam::Vec2 {
            x: self.x,
            y: self.y,
        }
    }
}
