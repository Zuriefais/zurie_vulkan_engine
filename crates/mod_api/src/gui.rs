use shared_types::{borsh::to_vec, GuiTextMessage};

use crate::string_to_pointer;

pub fn gui_text(message: GuiTextMessage) {
    let mut message_bin = to_vec(&message).unwrap();
    let len = message_bin.len() as u32;
    let ptr = message_bin.as_mut_ptr() as u32;
    unsafe { gui_text_sys(ptr, len) };
}
extern "C" {
    fn gui_text_sys(ptr: u32, len: u32);
}

pub fn gui_button(text: String) -> bool {
    let (ptr, len) = string_to_pointer(text);
    unsafe { gui_button_sys(ptr, len) != 0 }
}
extern "C" {
    fn gui_button_sys(ptr: u32, len: u32) -> i32;
}
