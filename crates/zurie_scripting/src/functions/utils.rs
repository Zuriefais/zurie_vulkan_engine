use crate::Host;
use log::{debug, error, info, trace, warn};

use super::ScriptingState;

impl Host for ScriptingState {
    fn info(&mut self, module_name: String, text: String) {
        info!(target: &module_name, "{}", text)
    }
    fn warn(&mut self, module_name: String, text: String) {
        warn!(target: &module_name, "{}", text)
    }
    fn error(&mut self, module_name: String, text: String) {
        error!(target: &module_name, "{}", text)
    }

    fn debug(&mut self, module_name: String, text: String) {
        debug!(target: &module_name, "{}", text)
    }

    fn trace(&mut self, module_name: String, text: String) {
        trace!(target: &module_name, "{}", text)
    }
}
