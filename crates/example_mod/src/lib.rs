use mod_api::{get_delta_time, get_mod_name_callback, info, string_to_pointer};

#[no_mangle]
pub extern "C" fn init() {
    info("initializing mod.....".to_string());
}

#[no_mangle]
pub extern "C" fn get_mod_name() {
    let (ptr, len) = string_to_pointer("example mod".to_string());
    unsafe { get_mod_name_callback(ptr, len) };
}

#[no_mangle]
pub extern "C" fn update() {
    info!("update..... delta_time: {}", get_delta_time());
}
