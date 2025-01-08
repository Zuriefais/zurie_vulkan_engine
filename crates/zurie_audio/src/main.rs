use std::{error::Error, io::stdin, path::Path};
use zurie_audio::AudioManager;

fn main() -> Result<(), Box<dyn Error>> {
    let mut manager = AudioManager::new();
    let sound = manager.load_sound(Path::new("static/sound.wav").into());
    loop {
        wait_for_enter_press()?;
        manager.play(sound);
    }
}

fn wait_for_enter_press() -> Result<(), Box<dyn Error>> {
    stdin().read_line(&mut "".into())?;
    Ok(())
}
