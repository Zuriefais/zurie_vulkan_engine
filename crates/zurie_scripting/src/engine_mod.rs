// use crate::{
//     functions::{
//         audio::register_audio_bindings,
//         camera::register_camera_bindings,
//         ecs::register_ecs_bindings,
//         events::{register_events_bindings, EventHandle, EventManager},
//         file::register_file_bindings,
//         game_logic::register_game_logic_bindings,
//         gui::{register_gui_button, register_gui_text},
//         input::{
//             register_key_pressed, register_request_mouse_pos, register_subscribe_for_key_event,
//         },
//         sprite::setup_sprite_bindings,
//         utils::register_utils_bindings,
//         ScriptingState, ZurieMod,
//     },
//     ModHandle,
// };
// use wasmtime_wasi::bindings::sync::Command;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

use crate::functions::events::EventManager;
use crate::functions::zurie::engine::core::EventHandle;
use crate::functions::ZurieMod;
use crate::ModHandle;
use crate::ScriptingState;
use anyhow::Ok;
use egui::Context;
use hashbrown::HashSet;
use log::info;
#[cfg(target_os = "android")]
use std::ffi::CString;
use std::sync::{Arc, RwLock};
use wasmtime::component::*;
use wasmtime::{Engine, Store};
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
use zurie_audio::AudioManager;
use zurie_ecs::World;
use zurie_render::sprite::SpriteManager;
use zurie_shared::slotmap::{Key, KeyData};
use zurie_types::{camera::Camera, glam::Vec2, KeyCode};
pub struct EngineMod {
    pub path: String,
    pub bindings: ZurieMod,
    pub store: Store<ScriptingState>,
    pub mod_name: Arc<RwLock<String>>,
    pub subscribed_keys: Arc<RwLock<HashSet<KeyCode>>>,
}

impl EngineMod {
    pub fn new(
        mod_path: String,
        engine: &Engine,
        gui_context: Context,
        pressed_keys_buffer: Arc<RwLock<HashSet<KeyCode>>>,
        mouse_pos: Arc<RwLock<Vec2>>,
        world: Arc<RwLock<World>>,
        camera: Arc<RwLock<Camera>>,
        event_manager: Arc<RwLock<EventManager>>,
        mod_handle: ModHandle,
        sprite_manager: Arc<RwLock<SpriteManager>>,
        audio_manager: AudioManager,

        #[cfg(target_os = "android")] android_app: AndroidApp,
    ) -> anyhow::Result<Self> {
        let engine = Engine::default();
        let component = Component::from_file(&engine, &mod_path)?;

        let mut linker: Linker<ScriptingState> = Linker::new(&engine);

        wasmtime_wasi::add_to_linker_sync(&mut linker)?;
        ZurieMod::add_to_linker(&mut linker, |state: &mut ScriptingState| state)?;
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args().build();
        let scripting_state = ScriptingState {
            audio_manager,
            wasi_ctx: wasi,
            resource_table: ResourceTable::new(),
            pressed_keys_buffer: pressed_keys_buffer.clone(),
            subscribed_keys: Default::default(),
            mouse_pos: mouse_pos.clone(),
        };

        let mut store = Store::new(&engine, scripting_state);

        let bindings = ZurieMod::instantiate(&mut store, &component, &linker)?;

        bindings.call_init(&mut store)?;
        Ok(Self {
            path: mod_path,
            bindings,
            store,
            mod_name: Default::default(),
            subscribed_keys: Default::default(),
        })
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        // self.update_fn.call(&mut self.store, ())?;
        Ok(())
    }

    pub fn key_event(&mut self, key_code: KeyCode) -> anyhow::Result<()> {
        let keys_lock = self.subscribed_keys.read().unwrap();
        if keys_lock.contains(&key_code) {
            self.bindings
                .call_key_event(&mut self.store, key_code as u32)?
        }
        Ok(())
    }

    pub fn scroll(&mut self, scroll: f32) -> anyhow::Result<()> {
        // self.scroll_fn.call(&mut self.store, scroll)?;
        Ok(())
    }

    pub fn handle_event(&mut self, event_handle: EventHandle, data: &[u8]) -> anyhow::Result<()> {
        // let mut store_mut = self.store.as_context_mut();
        // let memory = self
        //     .instance
        //     .get_memory(&mut store_mut, "memory")
        //     .ok_or_else(|| anyhow::anyhow!("failed to get memory"))?;
        // let ptr = self.alloc_fn.call(&mut self.store, data.len() as u32)? as usize;
        // memory
        //     .data_mut(&mut self.store)
        //     .get_mut(ptr..)
        //     .and_then(|slice| slice.get_mut(..data.len()))
        //     .ok_or_else(|| anyhow::anyhow!("failed to write to memory"))?
        //     .copy_from_slice(data);
        // self.event_fn
        //     .call(&mut self.store, KeyData::as_ffi(event_handle.data()))?;
        Ok(())
    }
}
