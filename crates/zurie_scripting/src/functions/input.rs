use log::info;
use zurie_input::InputState;
use zurie_types::KeyCode;

use super::{zurie::engine, ScriptingState};

impl engine::input::Host for ScriptingState {
    fn key_clicked(&mut self, key: u32) -> bool {
        self.input_state
            .pressed_keys()
            .contains(&KeyCode::try_from(key).unwrap())
    }

    fn subscribe_to_key_event(&mut self, key: u32) -> () {
        let key = KeyCode::try_from(key).unwrap();
        info!("Mod subscribe to {:?}", key);
        self.subscribed_keys.write().unwrap().insert(key);
    }

    #[doc = " Mouse"]
    fn mouse_pos(&mut self) -> engine::core::Vec2 {
        let vec = self.input_state.get_mouse_pos();
        engine::core::Vec2 { x: vec.x, y: vec.y }
    }

    fn left_mouse_clicked(&mut self) -> bool {
        self.input_state.left_mouse_button_pressed()
    }

    fn right_mouse_clicked(&mut self) -> bool {
        self.input_state.right_mouse_button_pressed()
    }
}
