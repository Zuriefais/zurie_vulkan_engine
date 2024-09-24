use mod_api::get_delta_time;

#[no_mangle]
pub extern "C" fn init() {
    println!("initializing mod.....");
}

#[no_mangle]
pub extern "C" fn update() {
    println!("update..... delta_time: {}", get_delta_time());
}
