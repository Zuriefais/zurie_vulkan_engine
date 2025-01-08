use kira::backend::cpal::CpalBackend;
use kira::sound::static_sound::StaticSoundData;
use kira::AudioManagerSettings;
use kira::DefaultBackend;
use log::info;
use log::warn;
use slotmap::{new_key_type, KeyData, SlotMap};

use std::str::MatchIndices;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Debug)]
pub enum AudioCommand {
    Play(SoundHandle),
    Load(Box<Path>, Sender<SoundHandle>),
    Stop,
}

pub struct AudioManager {
    manager: Sender<AudioCommand>,
}

impl AudioManager {
    pub fn new() -> AudioManager {
        let (sender, receiver) = channel();
        thread::spawn(move || {
            let mut audio_thread = AudioThread::new();
            audio_thread.run(receiver);
        });
        AudioManager { manager: sender }
    }

    pub fn load_sound(&self, path: Box<Path>) -> SoundHandle {
        let (sender, receiver) = channel();
        self.manager.send(AudioCommand::Load(path, sender));
        receiver.recv().unwrap()
    }

    pub fn play(&self, sound: SoundHandle) {
        self.manager.send(AudioCommand::Play(sound)).unwrap();
    }
}

new_key_type! { pub struct SoundHandle; }

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

    fn load_sound(&mut self, path: Box<Path>, sender: Sender<SoundHandle>) {
        self.sound_storage.insert_with_key(|key| {
            sender.send(key);
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
