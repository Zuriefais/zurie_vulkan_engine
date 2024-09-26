use mod_api::{
    get_delta_time, get_mod_name_callback, gui::gui_text, info, set_mod_name, string_to_pointer,
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
    info!(
        "update..... delta_time: {}, i: {}",
        get_delta_time(),
        unsafe { STATE.i }
    );
    gui_text(format!("GUI text from mod!!!"));
    gui_text(format!("GUI text from mod 2 time!!!, i: {}", unsafe {
        STATE.i
    }));
}
