extern "C" {
    fn get_delta_time_sys() -> f32;
}

pub fn get_delta_time() -> f32 {
    unsafe { return get_delta_time_sys() }
}
