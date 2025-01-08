use slotmap::Key;
use slotmap::KeyData;
use zurie_types::SoundHandle;

use crate::utils::string_to_pointer;

pub fn load_sound(path: String) -> SoundHandle {
    let (ptr, len) = string_to_pointer(path);
    KeyData::from_ffi(unsafe { load_sound_sys(ptr, len) }).into()
}

pub fn play_sound(sound: SoundHandle) {
    unsafe {
        play_sound_sys(KeyData::as_ffi(sound.data()));
    }
}

extern "C" {
    fn load_sound_sys(ptr: u32, len: u32) -> u64;
    fn play_sound_sys(sound: u64);
}
