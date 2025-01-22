#[cfg(target_os = "android")]
mod lib {
    use android_activity::{
        input::{InputEvent, KeyAction, KeyEvent, KeyMapChar, MotionAction},
        AndroidApp, InputStatus, MainEvent, PollEvent,
    };

    use log::info;

    use std::time::Duration;

    use winit::event_loop::{EventLoop, EventLoopBuilder};

    use winit::platform::android::EventLoopBuilderExtAndroid;

    use zurie_core::app::App;

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
}
