use std::ffi::CString;
pub mod gui;
pub use shared_types;
use shared_types::borsh::{to_vec, BorshSerialize};

pub fn get_delta_time() -> f32 {
    unsafe { get_delta_time_sys() }
}

pub fn string_to_pointer(s: String) -> (u32, u32) {
    let len = s.len() as u32;
    let cs = CString::new(s).unwrap();
    (cs.into_raw() as u32, len)
}

pub fn onj_to_pointer<T: BorshSerialize>(obj: &T) -> (u32, u32) {
    let mut message_bin = to_vec(obj).unwrap();
    let len = message_bin.len() as u32;
    let ptr = message_bin.as_mut_ptr() as u32;
    (ptr, len)
}

pub fn info(s: String) {
    let (ptr, len) = string_to_pointer(s);
    unsafe { info_sys(ptr, len) }
}

extern "C" {
    fn get_delta_time_sys() -> f32;

    fn info_sys(pointer: u32, len: u32);
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

#[macro_export]
macro_rules! set_mod_name {
    ($mod_name:literal) => {
        #[no_mangle]
        pub extern "C" fn get_mod_name() {
            let (ptr, len) = string_to_pointer($mod_name.to_string());
            unsafe { get_mod_name_callback(ptr, len) };
        }
    };
}
