use zurie_mod_api::ecs::*;
use zurie_mod_api::events::{emit_event_string, subscribe_to_event_by_name, EventHandle};
use zurie_mod_api::register_mod;
use zurie_mod_api::sprite::{load_sprite_from_buffer, load_sprite_from_file};
use zurie_mod_api::zurie_types::glam::Vec2;
use zurie_mod_api::zurie_types::serde::{Deserialize, Serialize};
use zurie_mod_api::zurie_types::ComponentData;
use zurie_mod_api::{
    gui::gui_text,
    input::{key_presed, subscribe_for_key_event},
    utils::*,
    zurie_types::{GuiTextMessage, KeyCode},
};

// Components
#[derive(Default)]
struct Position(Vec2);

#[derive(Default)]
struct Velocity(Vec2);

#[derive(Default, Deserialize, Serialize)]
struct Snake {
    segments: Vec<Entity>,
    growth_pending: bool,
}

#[derive(Default)]
struct Food;

#[derive(Default)]
struct Sprite(u64);

#[derive(Default)]
pub struct MyMod {
    snake_component: ComponentID,
    position_component: ComponentID,
    velocity_component: ComponentID,
    food_component: ComponentID,
    eat_apple_handle: EventHandle,
    snake_sprite: u64,
    apple_sprite: u64,
    move_timer: f32,
}

impl Mod for MyMod {
    fn update(&mut self) {
        self.move_timer += get_delta_time();

        // Handle input
        // self.handle_input();

        // // Update systems
        // if self.move_timer >= 0.2 {
        //     // Move every 0.2 seconds
        //     self.move_snake_system();
        //     self.move_timer = 0.0;
        // }

        // self.check_collision_system();
        self.render_ui_system();
    }

    fn init(&mut self) {
        // Register components
        self.position_component = register_component("position");
        self.velocity_component = register_component("velocity");
        self.snake_component = register_component("snake");
        self.food_component = register_component("food");

        // Load sprites
        self.snake_sprite =
            load_sprite_from_buffer(include_bytes!("../../../static/ase.aseprite").as_ref());
        self.apple_sprite = load_sprite_from_file("static/ase2.aseprite".into());

        // Subscribe to events
        self.eat_apple_handle = subscribe_to_event_by_name("eat_apple");

        // Subscribe to input
        for key in [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD] {
            subscribe_for_key_event(key);
        }

        // Spawn initial snake
        self.spawn_snake();
        self.spawn_food();
    }

    fn key_event(&mut self, key: KeyCode) {}

    fn scroll(&mut self, scroll: f32) {}

    fn event(&mut self, handle: EventHandle, data: &[u8]) {}

    fn new() -> Self
    where
        Self: Sized,
    {
        Self::default()
    }

    fn get_mod_name(&self) -> String {
        "Snake game".into()
    }

    // ... other required trait implementations ...
}

impl MyMod {
    fn spawn_snake(&self) {
        let head = spawn_entity();
        let mut segments = vec![head];

        // Set components for head
        head.set_component(
            self.position_component,
            ComponentData::Vector(Vec2::new(0.0, 0.0).into()),
        );
        head.set_component(
            self.velocity_component,
            ComponentData::Vector(Vec2::new(1.0, 0.0).into()),
        );
        head.set_sprite(self.snake_sprite);

        // Spawn initial tail segments
        for i in 1..3 {
            let segment = spawn_entity();
            segment.set_component(
                self.position_component,
                ComponentData::Vector(Vec2::new(-i as f32, 0.0).into()),
            );
            segment.set_sprite(self.snake_sprite);
            segments.push(segment);
        }

        // Set snake component with all segments
        head.set_component(
            self.snake_component,
            ComponentData::from_custom(Snake {
                segments,
                growth_pending: false,
            }),
        );
    }

    fn spawn_food(&self) {
        let food = spawn_entity();
        let pos = Vec2::new(get_rand_i32(-10, 10) as f32, get_rand_i32(-10, 10) as f32);

        food.set_component(self.position_component, ComponentData::Vector(pos.into()));
        food.set_component(self.food_component, ComponentData::None);
        food.set_sprite(self.apple_sprite);
    }

    fn handle_input(&mut self) {
        let new_velocity = if key_presed(KeyCode::KeyW) {
            Vec2::new(0.0, -1.0)
        } else if key_presed(KeyCode::KeyA) {
            Vec2::new(-1.0, 0.0)
        } else if key_presed(KeyCode::KeyS) {
            Vec2::new(0.0, 1.0)
        } else if key_presed(KeyCode::KeyD) {
            Vec2::new(1.0, 0.0)
        } else {
            return;
        };

        // Update snake head velocity
        if let Some(snake_entity) = get_entities_with_component(self.snake_component).first() {
            set_component(
                Entity(*snake_entity),
                self.velocity_component,
                ComponentData::Vector(new_velocity.into()),
            );
        }
    }

    fn move_snake_system(&self) {
        let snake_entities = get_entities_with_component(self.snake_component);
        if let Some(head_entity) = snake_entities.first() {
            if let Some(snake) =
                Entity(*head_entity).get_component_custom::<Snake>(self.snake_component)
            {
                // Update positions
                let mut previous_pos: Option<Vec2> = None;
                for &segment in &snake.segments {
                    if let Some(current_pos) = segment.get_component_vec2(self.position_component) {
                        if let Some(prev) = previous_pos {
                            segment.set_component(
                                self.position_component,
                                ComponentData::Vector(prev.into()),
                            );
                        } else if let Some(vel) =
                            segment.get_component_vec2(self.velocity_component)
                        {
                            let new_pos = current_pos + vel;
                            segment.set_component(
                                self.position_component,
                                ComponentData::Vector(new_pos.into()),
                            );
                        }
                        previous_pos = Some(current_pos);
                    }
                }
            }
        }
    }

    fn check_collision_system(&self) {
        let snake_entities = get_entities_with_component(self.snake_component);
        let food_entities = get_entities_with_component(self.food_component);

        if let (Some(head), Some(food)) = (snake_entities.first(), food_entities.first()) {
            if let (Some(head_pos), Some(food_pos)) = (
                Entity(*head).get_component_vec2(self.position_component),
                Entity(*food).get_component_vec2(self.position_component),
            ) {
                if head_pos == food_pos {
                    // Emit eat event
                    emit_event_string(self.eat_apple_handle, "Apple eaten".into());

                    // Mark snake for growth
                    if let Some(mut snake) =
                        Entity(*head).get_component_custom::<Snake>(self.snake_component)
                    {
                        snake.growth_pending = true;
                    }

                    // Despawn old food and spawn new
                    Entity(*food).despawn();
                    self.spawn_food();
                }
            }
        }
    }

    fn render_ui_system(&self) {
        gui_text(GuiTextMessage {
            window_title: "Game Stats".to_string(),
            label_text: format!("Snake Game - Frame Time: {} ms", get_delta_time() * 1000.0),
        });
    }
}

register_mod!(MyMod);
