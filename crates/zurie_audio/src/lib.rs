use kira::backend::cpal::CpalBackend;
use kira::AudioManagerSettings;
use kira::DefaultBackend;
use kira::{sound::static_sound::StaticSoundData, AudioManager};
use log::warn;
use slotmap::{new_key_type, KeyData, SlotMap};
use std::io::Cursor;
use std::path::Path;

pub struct EngineAudioManager {
    kira_manager: AudioManager,
    sound_storage: SlotMap<SoundHandle, StaticSoundData>,
}

new_key_type! { pub struct SoundHandle; }

impl EngineAudioManager {
    pub fn new() -> Self {
        EngineAudioManager {
            kira_manager: AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
                .unwrap(),
            sound_storage: Default::default(),
        }
    }

    pub fn load_sound(&mut self, load_info: SoundLoadInfo) -> anyhow::Result<SoundHandle> {
        let sound_data = match load_info {
            SoundLoadInfo::File(path) => StaticSoundData::from_file(path),
        }?;
        let handle = self.sound_storage.insert(sound_data);
        Ok(handle)
    }

    pub fn play(&mut self, sound: SoundHandle) {
        if let Some(sound) = self.sound_storage.get(sound) {
            self.kira_manager.play(sound.clone());
        } else {
            warn!("Could't play sound from handle")
        }
    }
}

pub enum SoundLoadInfo {
    File(Box<Path>),
}
