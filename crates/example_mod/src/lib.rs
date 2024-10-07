use mod_api::{
    get_delta_time, get_mod_name_callback,
    gui::{gui_button, gui_text},
    info, set_mod_name,
    shared_types::GuiTextMessage,
    string_to_pointer,
};

struct GameState {
    i: u32,
}

static mut STATE: GameState = GameState { i: 0 };

#[no_mangle]
pub extern "C" fn init() {
    info("initializing mod.....".to_string());
}

set_mod_name!("example_mod");

#[no_mangle]
pub extern "C" fn update() {
    unsafe { STATE.i += 1 }
    // info!(
    //     "update..... delta_time: {}, i: {}",
    //     get_delta_time(),
    //     unsafe { STATE.i }
    // );
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
        label_text: format!("test"),
    });
    if gui_button(GuiTextMessage {
        window_title: "Button test".to_string(),
        label_text: format!("Click me"),
    }) {
        info!("clicked!!!")
    };
}
