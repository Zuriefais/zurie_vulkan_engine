use std::ffi::CString;
pub mod gui;
pub mod input;
pub use zurie_types;
use zurie_types::bitcode::{self, Encode};

pub static mut PTR: u32 = 0;
pub static mut LEN: u32 = 0;

pub fn get_delta_time() -> f32 {
    unsafe { get_delta_time_sys() }
}

pub fn string_to_pointer(s: String) -> (u32, u32) {
    let len = s.len() as u32;
    let cs = CString::new(s).unwrap();
    (cs.into_raw() as u32, len)
}

pub fn obj_to_pointer<T: Encode>(obj: &T) -> (u32, u32) {
    let mut message_bin = bitcode::encode(obj);
    message_bin.shrink_to_fit();
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

#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    // create a new mutable buffer with capacity `len`
    let mut buf = Vec::with_capacity(len);
    // take a mutable pointer to the buffer
    let ptr = buf.as_mut_ptr();
    // take ownership of the memory block and
    // ensure that its destructor is not
    // called when the object goes out of scope
    // at the end of the function
    std::mem::forget(buf);
    unsafe {
        PTR = ptr as u32;
        LEN = len as u32
    }
    ptr
}
