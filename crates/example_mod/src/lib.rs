use zurie_mod_api::game_logic::{get_object_position, set_object_position, spawn_object};
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
    obj_0: u32,
    obj_1: u32,
}
fn move_obj(index: u32, direction: Vec2) {
    let mut obj_pos = get_object_position(index).expect("crash");
    obj_pos += direction;
    set_object_position(index, obj_pos)
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
            window_title: "obj 1 pos".to_string(),
            label_text: format!("pos: {}", get_object_position(self.obj_0).unwrap()),
        });
        if gui_button(GuiTextMessage {
            window_title: "Button test".to_string(),
            label_text: "Click me".to_string(),
        }) {
            info!("clicked!!!")
        };
        let mut direction = Vec2::ZERO;
        if key_presed(KeyCode::KeyW) {
            direction += Vec2 { x: 0.0, y: -0.1 };
        }
        if key_presed(KeyCode::KeyA) {
            direction += Vec2 { x: -0.1, y: 0.0 };
        }
        if key_presed(KeyCode::KeyS) {
            direction += Vec2 { x: 0.0, y: 0.1 };
        }
        if key_presed(KeyCode::KeyD) {
            direction += Vec2 { x: 0.1, y: 0.0 };
        }
        move_obj(self.obj_0, direction);
        info!("mouse pos: {:?}", get_mouse_pos());
    }

    fn key_event(&mut self, key: KeyCode) {
        info!("key clicked {:?}", key);
    }

    fn init(&mut self) {
        info("initializing mod.....".to_string());
        subscribe_for_key_event(KeyCode::KeyW);
        subscribe_for_key_event(KeyCode::KeyA);
        subscribe_for_key_event(KeyCode::KeyS);
        subscribe_for_key_event(KeyCode::KeyD);
        self.obj_0 = spawn_object(Object {
            position: Vec2::new(0.0, 0.0),
            scale: [1.0, 1.0],
            color: [1.0, 0.0, 1.0, 1.0],
        });
        self.obj_1 = spawn_object(Object {
            position: Vec2::new(2.0, 2.0),
            scale: [2.0, 2.0],
            color: [1.0, 0.0, 0.0, 1.0],
        });
    }
    fn get_mod_name(&self) -> String {
        "example_mod".to_string()
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            i: 0,
            obj_0: 0,
            obj_1: 0,
        }
    }
}

register_mod!(MyMod);
