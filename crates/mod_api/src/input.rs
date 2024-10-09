use shared_types::KeyCode;

pub fn subscribe_for_key_event(key: KeyCode) {
    unsafe {
        subscribe_for_key_event_sys(key);
    }
}

extern "C" {
    fn subscribe_for_key_event_sys(key: KeyCode);
}

pub fn if_key_presed(key: KeyCode) -> bool {
    return unsafe { if_key_pressed_sys(key) != 0 };
}

extern "C" {
    fn if_key_pressed_sys(key: KeyCode) -> i32;
}
