use crate::utils::{copy_obj_to_memory, get_obj_by_ptr, get_string_by_ptr};
use log::info;
use std::{
    path::Path,
    sync::{Arc, RwLock},
};
use wasmtime::{Linker, Store, ValType};
use zurie_audio::AudioManager;

use zurie_shared::slotmap::{new_key_type, Key, KeyData, SlotMap};
use zurie_types::SoundHandle;

pub fn register_audio_bindings(
    linker: &mut Linker<()>,
    store: &Store<()>,
    audio_manager: AudioManager,
) -> anyhow::Result<()> {
    register_load_sound(linker, store, audio_manager.clone())?;
    register_play_sound(linker, store, audio_manager.clone())?;

    Ok(())
}

fn register_load_sound(
    linker: &mut Linker<()>,
    store: &Store<()>,
    audio_manager: AudioManager,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "load_sound_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [wasmtime::ValType::I64].iter().cloned(),
        ),
        move |mut caller, params, results| {
            let path = get_string_by_ptr(
                &mut caller,
                params[0].unwrap_i32() as u32,
                params[1].unwrap_i32() as u32,
            )?;

            let handle = audio_manager.load_sound(path);
            results[0] = wasmtime::Val::I64(KeyData::as_ffi(handle.data()) as i64);
            Ok(())
        },
    )?;
    Ok(())
}

fn register_play_sound(
    linker: &mut Linker<()>,
    store: &Store<()>,
    audio_manager: AudioManager,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "play_sound_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I64].iter().cloned(),
            [].iter().cloned(),
        ),
        move |mut caller, params, results| {
            let sound: SoundHandle = KeyData::from_ffi(params[0].unwrap_i64() as u64).into();

            audio_manager.play(sound);
            Ok(())
        },
    )?;
    Ok(())
}
