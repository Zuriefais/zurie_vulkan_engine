use std::ffi::CString;
use zurie_types::{
    minicbor::{self, Decode, Encode},
    KeyCode,
};

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

pub fn obj_to_pointer<T: Encode<()>>(obj: &T) -> (u32, u32) {
    let mut message_bin = vec![];
    minicbor::encode(obj, &mut message_bin);
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
        use zurie_mod_api::utils::get_mod_name_callback;
        use zurie_mod_api::utils::string_to_pointer;
        #[no_mangle]
        pub extern "C" fn get_mod_name() {
            let (ptr, len) = string_to_pointer($mod_name.to_string());
            unsafe { get_mod_name_callback(ptr, len) };
        }
    };
}

pub fn get_obj_from_mem<T>() -> T
where
    T: for<'a> Decode<'a, ()>,
{
    let data = unsafe { Vec::from_raw_parts(PTR as *mut u8, LEN as usize, LEN as usize) };
    let obj = minicbor::decode::<T>(&data).unwrap();
    std::mem::drop(data);
    obj
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

pub trait Mod: Send + Sync {
    fn update(&mut self);
    fn key_event(&mut self, key: KeyCode);
    fn scroll(&mut self, scroll: f32);
    fn init(&mut self);

    fn new() -> Self
    where
        Self: Sized;
    fn get_mod_name(&self) -> String;
}

pub static mut MOD: Option<Box<dyn Mod>> = None;

#[macro_export]
macro_rules! register_mod {
    ($mod_type:ty) => {
        #[no_mangle]
        pub extern "C" fn new() {
            unsafe {
                MOD = Some(Box::new(<$mod_type as zurie_mod_api::utils::Mod>::new()));
            }
        }
    };
}

#[no_mangle]
pub extern "C" fn get_mod_name() {
    let game_mod = get_mod();
    let name = game_mod.get_mod_name();
    let (ptr, len) = string_to_pointer(name);
    unsafe { get_mod_name_callback(ptr, len) };
}

#[no_mangle]
pub extern "C" fn update() {
    let game_mod = get_mod();
    game_mod.update();
}

#[no_mangle]
pub extern "C" fn init() {
    let game_mod = get_mod();
    game_mod.init();
}

#[no_mangle]
pub extern "C" fn key_event(key_code: u32) {
    let game_mod = get_mod();
    game_mod.key_event(KeyCode::try_from(key_code).unwrap());
}

#[no_mangle]
pub extern "C" fn scroll(scroll: f32) {
    let game_mod = get_mod();
    game_mod.scroll(scroll);
}

pub fn get_mod() -> &'static mut dyn Mod {
    unsafe {
        #[allow(static_mut_refs)]
        MOD.as_deref_mut().unwrap()
    }
}

pub fn register_mod(build_mod: fn() -> Box<dyn Mod>) {
    unsafe { MOD = Some(build_mod()) }
}

extern "C" {
    fn get_rand_f32_sys(x: f32, y: f32) -> f32;
    fn get_rand_i32_sys(x: i32, y: i32) -> i32;
}

pub fn get_rand_f32(x: f32, y: f32) -> f32 {
    unsafe { get_rand_f32_sys(x, y) }
}
pub fn get_rand_i32(x: i32, y: i32) -> i32 {
    unsafe { get_rand_i32_sys(x, y) }
}
