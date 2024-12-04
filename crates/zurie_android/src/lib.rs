#[cfg(target_os = "android")]
use android_activity::{
    input::{InputEvent, KeyAction, KeyEvent, KeyMapChar, MotionAction},
    AndroidApp, InputStatus, MainEvent, PollEvent,
};
#[cfg(target_os = "android")]
use log::info;
#[cfg(target_os = "android")]
use std::time::Duration;
#[cfg(target_os = "android")]
use winit::event_loop::{EventLoop, EventLoopBuilder};
#[cfg(target_os = "android")]
use winit::platform::android::EventLoopBuilderExtAndroid;
#[cfg(target_os = "android")]
use zurie_core::app::App;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    info!("Android main called! 1 time");
    let event_loop = EventLoopBuilder::with_android_app(&mut EventLoopBuilder::default(), app)
        .build()
        .unwrap();
    let mut app = App::default();
    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("Event loop error: {}", e);
    }
}
