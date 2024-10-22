use zurie_mod_api::camera::{
    get_camera_position, get_zoom_factor, set_camera_position, set_zoom_factor,
};
use zurie_mod_api::game_logic::{spawn_object, ObjectHandle};
use zurie_mod_api::zurie_types::glam::Vec2;
use zurie_mod_api::zurie_types::{Object, Vector2};
use zurie_mod_api::{
    gui::{gui_button, gui_text},
    input::{key_presed, subscribe_for_key_event},
    utils::*,
    zurie_types::{GuiTextMessage, KeyCode},
};
use zurie_mod_api::{info, register_mod};

#[derive(Default)]
pub struct MyMod {
    i: u32,
    snake: Vec<ObjectHandle>,
    apple: ObjectHandle,
    direction: Vec2,
}

fn move_snake(snake: &mut Vec<ObjectHandle>, direction: Vec2, i: &mut u32, apple_pos: Vec2) {
    *i += 1;
    if *i == 10 {
        let new_pos = snake[0].get_pos().unwrap() + direction;
        if new_pos != apple_pos {
            let last_el = snake.pop().unwrap();
            last_el.set_pos(new_pos);
            snake.insert(0, last_el);
        } else {
            let new_obj = spawn_object(Object {
                position: Vector2::new(new_pos.x, new_pos.y),
                scale: [1.0, 1.0],
                color: [1.0, 0.0, 1.0, 1.0],
            });
            snake.insert(0, new_obj);
        }

        *i = 0;
    }
}

fn move_camera(direction: Vec2) {
    let new_cam_pos = get_camera_position() + direction;

    set_camera_position(new_cam_pos);
    info!(
        "cam pos: {}, cam_pos_expected: {}",
        get_camera_position(),
        new_cam_pos
    );
}

fn spawn_apple() -> ObjectHandle {
    let (x, y): (i32, i32) = (get_rand_i32(-10, 10), get_rand_i32(-10, 10));
    let position = Vector2::new(x as f32, y as f32);
    spawn_object(Object {
        position,
        scale: [1.0, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    })
}

impl Mod for MyMod {
    fn update(&mut self) {
        gui_text(GuiTextMessage {
            window_title: "Delta time".to_string(),
            label_text: format!("Delta time: {} s", get_delta_time()),
        });
        gui_text(GuiTextMessage {
            window_title: "Mod State".to_string(),
            label_text: format!("GUI text from mod 2 time!!!, i: {}", self.i),
        });
        gui_text(GuiTextMessage {
            window_title: "Snake props".to_string(),
            label_text: format!(
                "pos: {:?}, direction: {}, target pos: {:?}",
                self.snake[0].get_pos(),
                self.direction,
                self.apple.get_pos()
            ),
        });
        if gui_button(GuiTextMessage {
            window_title: "Button test".to_string(),
            label_text: "Click me".to_string(),
        }) {
            info!("clicked!!!")
        };

        if key_presed(KeyCode::KeyW) {
            self.direction = Vec2 { x: 0.0, y: -1.0 };
        }
        if key_presed(KeyCode::KeyA) {
            self.direction = Vec2 { x: -1.0, y: 0.0 };
        }
        if key_presed(KeyCode::KeyS) {
            self.direction = Vec2 { x: 0.0, y: 1.0 };
        }
        if key_presed(KeyCode::KeyD) {
            self.direction = Vec2 { x: 1.0, y: 0.0 };
        }
        let apple_pos = self.apple.get_pos().unwrap();
        move_snake(&mut self.snake, self.direction, &mut self.i, apple_pos);
        if self.snake[0].get_pos().unwrap() == apple_pos {
            self.apple.despawn();
            self.apple = spawn_apple();
        }
    }

    fn key_event(&mut self, key: KeyCode) {
        info!("key clicked {:?}", key);
    }

    fn init(&mut self) {
        info("initializing mod.....".to_string());
        subscribe_for_key_event(KeyCode::KeyW);
        subscribe_for_key_event(KeyCode::KeyA);
        subscribe_for_key_event(KeyCode::KeyS);
        subscribe_for_key_event(KeyCode::KeyD);
        self.snake.push(spawn_object(Object {
            position: Vector2::new(0.0, 0.0),
            scale: [1.0, 1.0],
            color: [1.0, 0.0, 1.0, 1.0],
        }));
        self.snake.push(spawn_object(Object {
            position: Vector2::new(1.0, 0.0),
            scale: [1.0, 1.0],
            color: [1.0, 0.0, 1.0, 1.0],
        }));
        self.snake.push(spawn_object(Object {
            position: Vector2::new(2.0, 0.0),
            scale: [1.0, 1.0],
            color: [1.0, 0.0, 1.0, 1.0],
        }));
        self.apple = spawn_apple();
    }
    fn get_mod_name(&self) -> String {
        "example_mod".to_string()
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            direction: match get_rand_i32(0, 3) {
                0 => Vec2::new(1.0, 0.0),  // Right
                1 => Vec2::new(-1.0, 0.0), // Left
                2 => Vec2::new(0.0, 1.0),  // Down
                _ => Vec2::new(0.0, -1.0), // Up (default case)
            },
            ..Default::default()
        }
    }

    fn scroll(&mut self, scroll: f32) {
        let zoom_factor = get_zoom_factor();
        if scroll > 0.0 && zoom_factor > 1.0 {
            set_zoom_factor(zoom_factor - 0.5);
        }
        if scroll < 1.0 {
            set_zoom_factor(zoom_factor + 0.5);
        }
        info!("zoom_factor: {}", zoom_factor);
    }
}

register_mod!(MyMod);
