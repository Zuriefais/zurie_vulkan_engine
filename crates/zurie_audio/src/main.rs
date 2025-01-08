use std::{error::Error, io::stdin, path::Path};

use kira::{
    backend::DefaultBackend, sound::static_sound::StaticSoundData, AudioManager,
    AudioManagerSettings,
};
use zurie_audio::EngineAudioManager;

fn main() -> Result<(), Box<dyn Error>> {
    let mut manager = EngineAudioManager::new();
    let sound = manager.load_sound(zurie_audio::SoundLoadInfo::File(
        Path::new("static/sound.wav").into(),
    ))?;
    loop {
        wait_for_enter_press()?;
        manager.play(sound);
    }
}

fn wait_for_enter_press() -> Result<(), Box<dyn Error>> {
    stdin().read_line(&mut "".into())?;
    Ok(())
}
