use crate::functions::zurie::engine::audio;
use crate::functions::zurie::engine::audio::*;

use crate::functions::KeyData;
use crate::ScriptingState;
use zurie_shared::slotmap::Key;

impl audio::Host for ScriptingState {
    fn load_sound(&mut self, path: String) -> SoundHandle {
        KeyData::as_ffi(self.audio_manager.load_sound(path).data())
    }

    fn play_sound(&mut self, handle: SoundHandle) {
        self.audio_manager.play(KeyData::from_ffi(handle).into());
    }
}
