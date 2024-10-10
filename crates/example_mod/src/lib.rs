use zurie_mod_api::{
    get_delta_time, get_mod_name_callback,
    gui::{gui_button, gui_text},
    info,
    input::{key_presed, subscribe_for_key_event},
    set_mod_name, string_to_pointer,
    zurie_types::{GuiTextMessage, KeyCode},
};

struct GameState {
    i: u32,
}

static mut STATE: GameState = GameState { i: 0 };

#[no_mangle]
pub extern "C" fn init() {
    info("initializing mod.....".to_string());
    subscribe_for_key_event(KeyCode::KeyW);
    subscribe_for_key_event(KeyCode::KeyA);
    subscribe_for_key_event(KeyCode::KeyS);
    subscribe_for_key_event(KeyCode::KeyD);
}

set_mod_name!("example_mod");

#[no_mangle]
pub extern "C" fn update() {
    unsafe { STATE.i += 1 }
    gui_text(GuiTextMessage {
        window_title: "Delta time".to_string(),
        label_text: format!("Delta time: {} s", get_delta_time()),
    });
    gui_text(GuiTextMessage {
        window_title: "Mod State".to_string(),
        label_text: format!("GUI text from mod 2 time!!!, i: {}", unsafe { STATE.i }),
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
}

#[no_mangle]
pub extern "C" fn key_event(key_code: u32) {
    info!("key clicked {:?}", KeyCode::try_from(key_code).unwrap())
}
