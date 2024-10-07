pub use borsh;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub struct GuiTextMessage {
    pub window_title: String,
    pub label_text: String,
}
