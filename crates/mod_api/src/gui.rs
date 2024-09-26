use crate::string_to_pointer;

pub fn gui_text(text: String) {
    let (ptr, len) = string_to_pointer(text);
    unsafe { gui_text_sys(ptr, len) };
}
extern "C" {
    fn gui_text_sys(ptr: u32, len: u32);
}