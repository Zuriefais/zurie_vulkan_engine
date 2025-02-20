use kira::AudioManagerSettings;
use kira::DefaultBackend;
use kira::backend::cpal::CpalBackend;
use kira::sound::static_sound::StaticSoundData;
use log::info;
use log::warn;
use slotmap::{KeyData, SlotMap, new_key_type};
use tracy_client::set_thread_name;

use std::str::MatchIndices;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::mpsc::channel;

use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread;
use zurie_types::SoundHandle;

#[derive(Debug)]
pub enum AudioCommand {
    Play(SoundHandle),
    Load(String, Sender<SoundHandle>),
    Stop,
}

#[derive(Clone)]
pub struct AudioManager {
    manager: Sender<AudioCommand>,
}

impl AudioManager {
    pub fn new() -> AudioManager {
        let (sender, receiver) = channel();
        thread::spawn(move || {
            set_thread_name!("Audio thread");
            let mut audio_thread = AudioThread::new();
            audio_thread.run(receiver);
        });
        AudioManager { manager: sender }
    }

    pub fn load_sound(&self, path: String) -> SoundHandle {
        let (sender, receiver) = channel();
        self.manager.send(AudioCommand::Load(path, sender));
        receiver.recv().unwrap()
    }

    pub fn play(&self, sound: SoundHandle) {
        self.manager.send(AudioCommand::Play(sound)).unwrap();
    }
}

pub struct AudioThread {
    kira_manager: kira::AudioManager,
    sound_storage: SlotMap<SoundHandle, StaticSoundData>,
}

impl AudioThread {
    pub fn new() -> Self {
        AudioThread {
            kira_manager:
                kira::AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
            sound_storage: Default::default(),
        }
    }

    fn run(&mut self, receiver: Receiver<AudioCommand>) {
        while let Ok(command) = receiver.recv() {
            info!("command received, {:?}", command);
            match command {
                AudioCommand::Play(sound_handle) => self.play(sound_handle),
                AudioCommand::Load(path, sender) => self.load_sound(path, sender),
                AudioCommand::Stop => break,
            }
        }
    }

    fn load_sound(&mut self, path: String, sender: Sender<SoundHandle>) {
        self.sound_storage.insert_with_key(|key| {
            sender.send(key);
            info!("Loading sound with handle: {:?}", key);
            StaticSoundData::from_file(path).expect("error loading file")
        });
    }

    fn play(&mut self, sound: SoundHandle) {
        if let Some(sound) = self.sound_storage.get(sound) {
            self.kira_manager.play(sound.clone());
        } else {
            warn!("Could't play sound from handle")
        }
    }
}
