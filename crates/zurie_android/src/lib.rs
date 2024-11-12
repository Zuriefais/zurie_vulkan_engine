use android_activity::{
    input::{InputEvent, KeyAction, KeyEvent, KeyMapChar, MotionAction},
    AndroidApp, InputStatus, MainEvent, PollEvent,
};
use log::info;
use std::time::Duration;

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    info!("Android main called! 1 time");
    loop {
        app.poll_events(Some(Duration::from_millis(10)), |event| match event {
            PollEvent::Wake => {
                info!("Wake event received");
            }
            PollEvent::Main(main_event) => match main_event {
                MainEvent::InitWindow { .. } => {
                    info!("Window initialized");
                }
                MainEvent::TerminateWindow { .. } => {
                    info!("Window terminated");
                }
                MainEvent::WindowResized { .. } => {
                    info!("Window resized");
                }
                MainEvent::Resume { .. } => {
                    info!("App resumed");
                }
                MainEvent::Pause { .. } => {
                    info!("App paused");
                }
                MainEvent::Destroy { .. } => {
                    info!("App destroyed");
                }
                _ => {}
            },
            PollEvent::Timeout => {}
            _ => {}
        });
    }
}
