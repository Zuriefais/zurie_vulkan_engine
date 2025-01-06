use zurie_mod_api::{register_mod, utils::Mod};

pub struct VampireMod {}
impl Mod for VampireMod {
    fn update(&mut self) {
        todo!()
    }

    fn key_event(&mut self, key: zurie_mod_api::zurie_types::KeyCode) {
        todo!()
    }

    fn scroll(&mut self, scroll: f32) {
        todo!()
    }

    fn init(&mut self) {
        todo!()
    }

    fn event(&mut self, handle: zurie_mod_api::events::EventHandle, data: &[u8]) {
        todo!()
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        todo!()
    }

    fn get_mod_name(&self) -> String {
        todo!()
    }
}

register_mod!(VampireMod);
