use std::{error::Error, io::stdin};

use kira::{
    backend::DefaultBackend, sound::static_sound::StaticSoundData, AudioManager,
    AudioManagerSettings,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;
    let sound_data = StaticSoundData::from_file("static/sound.wav")?;

    println!("Press enter to play a sound");
    loop {
        wait_for_enter_press()?;
        manager.play(sound_data.clone())?;
    }
}

fn wait_for_enter_press() -> Result<(), Box<dyn Error>> {
    stdin().read_line(&mut "".into())?;
    Ok(())
}
