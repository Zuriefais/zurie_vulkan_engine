use android_activity::{
    input::{InputEvent, KeyAction, KeyEvent, KeyMapChar, MotionAction},
    AndroidApp, InputStatus, MainEvent, PollEvent,
};
use log::info;

#[no_mangle]

pub unsafe extern "C" fn ANativeActivity_onCreate(
    activity: *mut ndk_sys::ANativeActivity,
    _saved_state: *mut std::os::raw::c_void,
    _saved_state_size: usize,
) {
}

#[no_mangle]
fn android_main(app: AndroidApp) {}
