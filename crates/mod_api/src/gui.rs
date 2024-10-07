use shared_types::GuiTextMessage;

use crate::onj_to_pointer;

pub fn gui_text(message: GuiTextMessage) {
    let (ptr, len) = onj_to_pointer(&message);
    unsafe { gui_text_sys(ptr, len) };
}
extern "C" {
    fn gui_text_sys(ptr: u32, len: u32);
}

pub fn gui_button(message: GuiTextMessage) -> bool {
    let (ptr, len) = onj_to_pointer(&message);
    unsafe { gui_button_sys(ptr, len) != 0 }
}
extern "C" {
    fn gui_button_sys(ptr: u32, len: u32) -> i32;
}
