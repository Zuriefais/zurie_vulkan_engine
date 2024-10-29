use crate::utils::{string_to_pointer, LEN, PTR};

pub fn load_file(path: String) -> Option<Vec<u8>> {
    let (ptr, len) = string_to_pointer(path);
    let loaded = unsafe { load_file_sys(ptr, len) };
    if loaded == 1 {
        Some(unsafe { Vec::from_raw_parts(PTR as *mut u8, LEN as usize, LEN as usize) })
    } else {
        None
    }
}

pub fn safe_file(path: String, data: &[u8]) {
    let (path_ptr, path_len) = string_to_pointer(path);
    let (file_ptr, file_len) = (data.as_ptr() as u32, data.len() as u32);
    unsafe { safe_file_sys(path_ptr, path_len, file_ptr, file_len) }
}

extern "C" {
    fn load_file_sys(ptr: u32, len: u32) -> i32;
    fn safe_file_sys(path_ptr: u32, path_len: u32, file_ptr: u32, file_len: u32);
}
