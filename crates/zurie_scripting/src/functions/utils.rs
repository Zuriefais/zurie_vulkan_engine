use crate::Host;
use log::{debug, error, info, trace, warn};

use super::ScriptingState;

impl Host for ScriptingState {
    fn info(&mut self, text: String) {
        info!("{}", text)
    }
    fn warn(&mut self, text: String) {
        warn!("{}", text)
    }
    fn error(&mut self, text: String) {
        error!("{}", text)
    }

    fn debug(&mut self, text: String) {
        debug!("{}", text)
    }

    fn trace(&mut self, text: String) {
        trace!("{}", text)
    }
}
