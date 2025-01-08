use zurie_mod_api::sprite::load_sprite_from_file;
use zurie_mod_api::utils::MOD;
use zurie_mod_api::{
    ecs::{register_pos_component, spawn_entity},
    register_mod,
    utils::Mod,
    zurie_types::{glam::Vec2, ComponentData},
};

pub struct VampireMod {}
impl Mod for VampireMod {
    fn update(&mut self) {}

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
    }

    fn event(&mut self, handle: zurie_mod_api::events::EventHandle, data: &[u8]) {}

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn get_mod_name(&self) -> String {
        "vampire_like_demo".into()
    }
}

register_mod!(VampireMod);
