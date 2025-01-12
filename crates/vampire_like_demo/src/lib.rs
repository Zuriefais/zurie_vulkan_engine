use zurie_mod_interface::{ZurieMod, register_zurie_mod};

pub struct Game;

impl ZurieMod for Game {
    fn new() -> Self {
        Game {}
    }

    fn get_mod_name(&self) -> String {
        "Vampire like demo".into()
    }
}

register_zurie_mod!(Game);
