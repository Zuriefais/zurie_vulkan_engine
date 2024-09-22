#[no_mangle]
pub extern "C" fn init() {
    println!("initializing mod.....");
}

#[no_mangle]
pub extern "C" fn update(delta_time: f32) {
    println!("update..... delta_time: {}", delta_time);
}
