use std::sync::Arc;
use std::sync::RwLock;

use crate::functions::zurie::engine::audio::SoundHandle;

use egui::Context;
use egui::Ui;
use egui::Window;
use hashbrown::HashSet;
use wasmtime::component::bindgen;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::WasiCtx;
use zurie::engine::gui::WidgetResponse;
use zurie_audio::AudioManager;
use zurie_ecs::World;
use zurie_event::EventManager;
use zurie_shared::slotmap::{new_key_type, Key, KeyData, SlotMap};
pub mod audio;
pub mod camera;
pub mod ecs;
pub mod events;
pub mod game_logic;
pub mod gui;
pub mod input;
pub mod sprite;
pub mod utils;
use wasmtime_wasi::WasiView;
use zurie_types::camera::Camera;
use zurie_types::glam::Vec2;
use zurie_types::KeyCode;
use zurie_types::ModHandle;

bindgen!("zurie-mod" in "zurie_engine.wit");

pub struct ScriptingState {
    //GUI
    pub gui_context: Context,
    pub windows: Vec<Vec<WidgetResponse>>,

    //ECS
    pub world: Arc<RwLock<World>>,

    //Audio
    pub audio_manager: AudioManager,

    //Input
    pub pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
    pub subscribed_keys: Arc<RwLock<HashSet<KeyCode>>>,
    pub mouse_pos: Arc<RwLock<Vec2>>,

    //Camera
    pub camera: Arc<RwLock<Camera>>,

    //Event
    pub event_manager: Arc<RwLock<EventManager>>,
    pub mod_handle: ModHandle,

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
