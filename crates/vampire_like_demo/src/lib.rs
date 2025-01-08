use zurie_mod_api::audio::{load_sound, play_sound};
use zurie_mod_api::input::key_presed;
use zurie_mod_api::sprite::load_sprite_from_file;
use zurie_mod_api::utils::MOD;
use zurie_mod_api::zurie_types::SoundHandle;
use zurie_mod_api::{
    ecs::{register_pos_component, spawn_entity},
    register_mod,
    utils::Mod,
    zurie_types::{glam::Vec2, ComponentData},
};

#[derive(Default)]
pub struct VampireMod {
    sound: SoundHandle,
}
impl Mod for VampireMod {
    fn update(&mut self) {
        if key_presed(zurie_mod_api::zurie_types::KeyCode::Enter) {
            play_sound(self.sound);
        }
    }

    fn key_event(&mut self, key: zurie_mod_api::zurie_types::KeyCode) {}

    fn scroll(&mut self, scroll: f32) {}

    fn init(&mut self) {
        let pos_component = register_pos_component();
        let entity = spawn_entity();
        let sprite = load_sprite_from_file("static/ase2.aseprite".into());
        entity.set_component(
            pos_component,
            ComponentData::Vector(Vec2::new(0.0, 0.0).into()),
        );
        entity.set_sprite(sprite);
        self.sound = load_sound("static/sound.wav".into());
    }

    fn event(&mut self, handle: zurie_mod_api::events::EventHandle, data: &[u8]) {}

    fn new() -> Self
    where
        Self: Sized,
    {
        Default::default()
    }

    fn get_mod_name(&self) -> String {
        "vampire_like_demo".into()
    }
}

register_mod!(VampireMod);
