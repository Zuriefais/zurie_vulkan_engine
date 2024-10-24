use zurie_types::GuiTextMessage;

use crate::utils::obj_to_pointer;

pub fn gui_text(message: GuiTextMessage) {
    // Create a temporary copy to ensure proper encoding
    let msg = GuiTextMessage {
        window_title: message.window_title.clone(),
        label_text: message.label_text.clone(),
    };

    let (ptr, len) = obj_to_pointer(&msg);
    unsafe { gui_text_sys(ptr, len) };
}
extern "C" {
    fn gui_text_sys(ptr: u32, len: u32);
}

pub fn gui_button(message: GuiTextMessage) -> bool {
    let (ptr, len) = obj_to_pointer(&message);
    unsafe { gui_button_sys(ptr, len) != 0 }
}
extern "C" {
    fn gui_button_sys(ptr: u32, len: u32) -> i32;
}
