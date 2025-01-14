use zurie_mod_interface::{
    ZurieMod,
    engine::{
        audio::{load_sound, play_sound},
        ecs::spawn_entity,
    },
    input::subscribe_to_key_event,
    log::info,
    register_zurie_mod,
};

pub struct Game {
    sound: u64,
}

impl ZurieMod for Game {
    fn new() -> Self {
        Game { sound: 0 }
    }

    fn get_mod_name(&self) -> String {
        "Vampire like demo".into()
    }

    fn init(&mut self) {
        subscribe_to_key_event(zurie_mod_interface::input::KeyCode::KeyW);
        self.sound = load_sound("static/sound.wav");
        spawn_entity();
        info!("Mod inited!!")
    }

    fn key_event(&mut self, key: zurie_mod_interface::input::KeyCode) {
        if key == zurie_mod_interface::input::KeyCode::KeyW {
            play_sound(self.sound);
        }
    }

    fn update(&mut self) {}
}

register_zurie_mod!(Game);
