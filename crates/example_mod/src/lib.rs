use zurie_mod_api::game_logic::spawn_object;
use zurie_mod_api::zurie_types::glam::Vec2;
use zurie_mod_api::zurie_types::Object;
use zurie_mod_api::{
    gui::{gui_button, gui_text},
    input::{get_mouse_pos, key_presed, subscribe_for_key_event},
    utils::*,
    zurie_types::{GuiTextMessage, KeyCode},
};
use zurie_mod_api::{info, register_mod};

pub struct MyMod {
    i: u32,
}

impl Mod for MyMod {
    fn update(&mut self) {
        self.i += 1;
        gui_text(GuiTextMessage {
            window_title: "Delta time".to_string(),
            label_text: format!("Delta time: {} s", get_delta_time()),
        });
        gui_text(GuiTextMessage {
            window_title: "Mod State".to_string(),
            label_text: format!("GUI text from mod 2 time!!!, i: {}", self.i),
        });
        gui_text(GuiTextMessage {
            window_title: "Test".to_string(),
            label_text: "test".to_string(),
        });
        if gui_button(GuiTextMessage {
            window_title: "Button test".to_string(),
            label_text: "Click me".to_string(),
        }) {
            info!("clicked!!!")
        };
        if key_presed(KeyCode::KeyW) {
            info!("key w pressed")
        }
        info!("mouse pos: {:?}", get_mouse_pos())
    }

    fn key_event(&mut self, key: KeyCode) {
        info!("key clicked {:?}", key)
    }

    fn init(&mut self) {
        info("initializing mod.....".to_string());
        subscribe_for_key_event(KeyCode::KeyW);
        subscribe_for_key_event(KeyCode::KeyA);
        subscribe_for_key_event(KeyCode::KeyS);
        subscribe_for_key_event(KeyCode::KeyD);
        spawn_object(Object {
            position: Vec2::new(10.0, 10.0),
        });
    }
    fn get_mod_name(&self) -> String {
        "example_mod".to_string()
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        Self { i: 0 }
    }
}

register_mod!(MyMod);
