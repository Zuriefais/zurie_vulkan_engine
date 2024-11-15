use zurie_mod_api::camera::{get_zoom_factor, set_zoom_factor};
use zurie_mod_api::ecs::*;
use zurie_mod_api::ecs::{register_component, spawn_entity};
use zurie_mod_api::events::{emit_event_string, subscribe_to_event_by_name, EventHandle};
use zurie_mod_api::game_logic::{spawn_object, ObjectHandle};
use zurie_mod_api::zurie_types::glam::Vec2;
use zurie_mod_api::zurie_types::ComponentData;
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
    eat_apple_handle: EventHandle,
    direction: Vec2,
}

fn move_snake(snake: &mut Vec<ObjectHandle>, direction: Vec2, i: &mut u32, apple_pos: Vec2) {
    *i += 1;
    if *i == 10 {
        // Get head position safely
        if let Some(head_pos) = snake[0].get_pos() {
            let new_pos = head_pos + direction;

            if new_pos != apple_pos {
                if let Some(last_el) = snake.pop() {
                    last_el.set_pos(new_pos);
                    snake.insert(0, last_el);
                }
            } else {
                let new_obj = spawn_object(Object {
                    position: Vector2::new(new_pos.x, new_pos.y),
                    scale: [1.0, 1.0],
                    color: [1.0, 0.0, 1.0, 1.0],
                });
                snake.insert(0, new_obj);
            }
        }
        *i = 0;
    }
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
        // Add safety checks for snake array
        if self.snake.is_empty() {
            return;
        }

        gui_text(GuiTextMessage {
            window_title: "Delta time".to_string(),
            label_text: format!("Delta time: {} s", get_delta_time()),
        });

        gui_text(GuiTextMessage {
            window_title: "Mod State".to_string(),
            label_text: format!("GUI text from mod 2 time!!!, i: {}", self.i),
        });

        // Safely get snake head position
        let head_pos = match self.snake[0].get_pos() {
            Some(pos) => pos,
            None => return,
        };

        // Safely get apple position
        let apple_pos = match self.apple.get_pos() {
            Some(pos) => pos,
            None => return,
        };

        gui_text(GuiTextMessage {
            window_title: "Snake props".to_string(),
            label_text: format!(
                "pos: {:?}, direction: {}, target pos: {:?}",
                head_pos, self.direction, apple_pos
            ),
        });

        if gui_button(GuiTextMessage {
            window_title: "Button test".to_string(),
            label_text: "Click me".to_string(),
        }) {
            info!("clicked!!!")
        };

        // Handle movement input
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

        move_snake(&mut self.snake, self.direction, &mut self.i, apple_pos);

        // Check collision with apple
        if let Some(head_pos) = self.snake[0].get_pos() {
            if head_pos == apple_pos {
                emit_event_string(self.eat_apple_handle, "Apple eated".into())
            }
        }
    }

    fn key_event(&mut self, key: KeyCode) {
        info!("key clicked {:?}", key);
    }

    fn init(&mut self) {
        self.eat_apple_handle = subscribe_to_event_by_name("eat_apple");
        let component = register_component("Component");
        let entity = spawn_entity();
        entity.set_component(
            component,
            ComponentData::Vector(Vec2 { x: 10.0, y: 10.0 }.into()),
        );
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

    fn event(&mut self, handle: EventHandle, _: &[u8]) {
        info!("event catched: {:?}", handle);
        if handle == self.eat_apple_handle {
            self.apple.despawn();
            self.apple = spawn_apple();
        }
    }
}

register_mod!(MyMod);
