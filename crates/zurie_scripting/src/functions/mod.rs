use crate::functions::zurie::engine::audio::SoundHandle;
use wasmtime::component::bindgen;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::WasiCtx;
use zurie_audio::AudioManager;
use zurie_shared::slotmap::{new_key_type, Key, KeyData, SlotMap};
pub mod audio;
pub mod camera;
pub mod ecs;
pub mod events;
pub mod file;
pub mod game_logic;
pub mod gui;
pub mod input;
pub mod sprite;
pub mod utils;
use wasmtime_wasi::WasiView;

bindgen!("zurie-mod" in "zurie_engine.wit");

pub struct ScriptingState {
    pub audio_manager: AudioManager,

    //Wasi spacific fields
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
}

impl WasiView for ScriptingState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

use crate::Host;
use log::{debug, error, info, trace, warn};

impl Host for ScriptingState {
    fn info(&mut self, text: String) {
        info!("{}", text)
    }
    fn warn(&mut self, text: String) {
        warn!("{}", text)
    }
    fn error(&mut self, text: String) {
        error!("{}", text)
    }

    fn debug(&mut self, text: String) {
        debug!("{}", text)
    }

    fn trace(&mut self, text: String) {
        trace!("{}", text)
    }
}
