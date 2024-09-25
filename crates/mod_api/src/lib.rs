use std::ffi::CString;

extern "C" {
    fn get_delta_time_sys() -> f32;
}

pub fn get_delta_time() -> f32 {
    unsafe { get_delta_time_sys() }
}

pub fn string_to_pointer(s: String) -> (u32, u32) {
    let len = s.len() as u32;
    let cs = CString::new(s).unwrap();
    return (cs.into_raw() as u32, len);
}

pub fn info(s: String) {
    let (ptr, len) = string_to_pointer(s);
    unsafe { info_sys(ptr, len) }
}
extern "C" {
    pub fn get_mod_name_callback(ptr: u32, len: u32);
}

#[macro_export]
macro_rules! info {
    () => {
        info("\n")
    };
    ($($arg:tt)*) => {{
        info(format!($($arg)*));
    }};
}

extern "C" {
    fn info_sys(pointer: u32, len: u32);
}
