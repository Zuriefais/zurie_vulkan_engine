use functions::ScriptingState;
use zurie_shared::slotmap::new_key_type;

pub mod engine_mod;
pub mod functions;
pub mod mod_manager;
pub mod utils;

new_key_type! {
    pub struct ModHandle;
}
use crate::functions::zurie::engine::core::Host;
