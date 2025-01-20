pub mod audio;
pub mod camera;
pub mod ecs;
pub mod events;
pub mod gui;
pub mod input;
pub mod rand;
pub mod sprite;
pub mod utils;

use crate::functions::zurie::engine::audio::SoundHandle;
use egui::{Context, Ui, Window};
use hashbrown::HashSet;
use std::sync::{Arc, RwLock};
use wasmtime::component::{bindgen, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiView};
use zurie::engine::gui::WidgetResponse;
use zurie_audio::AudioManager;
use zurie_ecs::ComponentID;
use zurie_ecs::World;
use zurie_event::EventManager;
use zurie_input::InputState;
use zurie_render::sprite::SpriteManager;
use zurie_shared::slotmap::{new_key_type, Key, KeyData, SlotMap};
use zurie_types::camera::Camera;
use zurie_types::glam::Vec2;
use zurie_types::KeyCode;
use zurie_types::ModHandle;

bindgen!("zurie-mod" in "zurie_engine.wit");

pub struct ScriptingState {
    //Sprite
    pub sprite_manager: Arc<RwLock<SpriteManager>>,
    pub sprite_component: ComponentID,

    //GUI
    pub gui_context: Context,
    pub windows: Vec<Vec<WidgetResponse>>,

    //ECS
    pub world: Arc<RwLock<World>>,

    //Audio
    pub audio_manager: AudioManager,

    //Input
    pub subscribed_keys: Arc<RwLock<HashSet<KeyCode>>>,
    pub input_state: InputState,

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
