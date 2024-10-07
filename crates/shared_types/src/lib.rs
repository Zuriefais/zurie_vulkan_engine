pub use bitcode;
use bitcode::{Decode, Encode};

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct GuiTextMessage {
    pub window_title: String,
    pub label_text: String,
}
