use mod_api::{get_delta_time, get_mod_name_callback, info, set_mod_name, string_to_pointer};

#[no_mangle]
pub extern "C" fn init() {
    info("initializing mod.....".to_string());
}

set_mod_name!("example_mod");

#[no_mangle]
pub extern "C" fn update() {
    info!("update..... delta_time: {}", get_delta_time());
}
