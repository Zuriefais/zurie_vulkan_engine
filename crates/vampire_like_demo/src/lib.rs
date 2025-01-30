use std::iter;
use std::ops::Range;
use zurie_mod_interface::ecs::{get_entities_with_component, get_entities_with_components};
use zurie_mod_interface::engine::camera::set_zoom;
use zurie_mod_interface::engine::core::{ComponentId, SpriteHandle};
use zurie_mod_interface::engine::input::left_mouse_clicked;
use zurie_mod_interface::engine::sprite::{load_sprite_bin, load_sprite_file, set_sprite};
use zurie_mod_interface::{
    ZurieMod,
    ecs::Entity,
    engine::{
        audio::{load_sound, play_sound},
        camera::get_zoom,
        ecs::{ComponentData, register_component, spawn_entity},
        gui::{Widget, WidgetResponse, create_window},
    },
    glam::{self, Vec2},
    input::{key_clicked, subscribe_to_key_event},
    log::info,
    register_zurie_mod,
};

#[derive(Default)]
pub struct Game {
    sound: u64,
    player: Entity,
    pos_component: u64,
    test_sprite: u64,
    enemy_component: u64,
}

impl ZurieMod for Game {
    fn new() -> Self {
        Game::default()
    }

    fn get_mod_name(&self) -> String {
        "Vampire like demo".into()
    }

    fn init(&mut self) {
        subscribe_to_key_event(zurie_mod_interface::input::KeyCode::KeyO);
        self.sound = load_sound("static/sound.wav");

        // Load all sprites
        let player_sprite = load_sprite_bin(include_bytes!("../../../static/player.aseprite"));
        let enemy_sprite = load_sprite_bin(include_bytes!("../../../static/error.aseprite"));
        self.test_sprite = load_sprite_bin(include_bytes!("../../../static/ase2.aseprite"));

        // Initialize entities and components
        let player_ent = Entity::spawn();
        let pos_component = register_component("position");
        let enemy_component = register_component("enemy");
        self.enemy_component = enemy_component;

        info!("enemy component: {}", enemy_component);
        player_ent.set_component(
            pos_component,
            ComponentData::Vec2(zurie_mod_interface::engine::core::Vec2 { x: 0.0, y: 0.0 }),
        );
        self.player = player_ent;
        self.pos_component = pos_component;
        self.player.set_sprite(player_sprite);

        // Spawn enemies
        spawn_enemies(
            enemy_component,
            pos_component,
            Vec2::new(0.0, 0.0).into(),
            enemy_sprite,
        );

        // Set initial zoom
        set_zoom(15.0);
        info!("Mod inited!!");
    }

    fn key_event(&mut self, key: zurie_mod_interface::input::KeyCode) {
        if key == zurie_mod_interface::input::KeyCode::KeyO {
            play_sound(self.sound);
        }
    }

    fn update(&mut self) {
        let direction = [
            (
                key_clicked(zurie_mod_interface::input::KeyCode::KeyW),
                Vec2::new(0.0, -1.0),
            ),
            (
                key_clicked(zurie_mod_interface::input::KeyCode::KeyA),
                Vec2::new(-1.0, 0.0),
            ),
            (
                key_clicked(zurie_mod_interface::input::KeyCode::KeyS),
                Vec2::new(0.0, 1.0),
            ),
            (
                key_clicked(zurie_mod_interface::input::KeyCode::KeyD),
                Vec2::new(1.0, 0.0),
            ),
        ]
        .iter()
        .filter(|(key, _)| *key)
        .map(|(_, dir)| *dir)
        .fold(Vec2::ZERO, |acc, dir| acc + dir);

        // Update player position
        let player_pos = self
            .player
            .get_component(self.pos_component)
            .and_then(|old_pos| {
                if let ComponentData::Vec2(old_pos) = old_pos {
                    let new_pos: Vec2 = Into::<Vec2>::into(old_pos) + direction * 0.1;
                    self.player
                        .set_component(self.pos_component, ComponentData::Vec2(new_pos.into()));
                    Some(new_pos)
                } else {
                    Some(Vec2::ZERO)
                }
            })
            .unwrap_or(Vec2::ZERO);

        // Move enemies towards player
        move_enemies(self.pos_component, self.enemy_component, player_pos);

        // Handle GUI window
        let widgets = vec![
            Widget::Label("My custom label in window".into()),
            Widget::Button("My custom button. Try to click me".into()),
        ];
        let responses = create_window("My window", &widgets);
        if let Some(WidgetResponse::Clicked(true)) = responses.get(1) {
            info!("Mouse clicked");
        }
    }

    fn scroll(&mut self, amount: f32) {
        let zoom = get_zoom() + amount;
        set_zoom(zoom);
    }
}

fn spawn_enemies(
    enemy_component: ComponentId,
    pos_component: ComponentId,
    player_pos: Vec2,
    sprite: SpriteHandle,
) {
    let positions = [
        Vec2::new(1.0, 1.0),
        Vec2::new(1.0, -1.0),
        Vec2::new(-1.0, 1.0),
        Vec2::new(-1.0, -1.0),
    ];

    positions
        .iter()
        .for_each(|&pos| spawn_enemy(enemy_component, pos_component, pos, sprite));
}

fn spawn_enemy(
    enemy_component: ComponentId,
    pos_component: ComponentId,
    enemy_pos: Vec2,
    sprite: SpriteHandle,
) {
    Entity::spawn()
        .set_component(pos_component, ComponentData::Vec2(enemy_pos.into()))
        .set_sprite(sprite)
        .set_component(enemy_component, ComponentData::None);
}

fn move_enemies(pos_component: ComponentId, enemy_component: ComponentId, player_pos: Vec2) {
    let enemies = get_entities_with_component(enemy_component);
    info!("moving enemies: {:?}", &enemies);

    for enemy in enemies.iter() {
        if let Some(ComponentData::Vec2(enemy_pos)) = enemy.get_component(pos_component) {
            let new_enemy_pos: Vec2 = Into::<Vec2>::into(enemy_pos)
                + vector_between_coordinates(enemy_pos.into(), player_pos);
            enemy.set_component(pos_component, ComponentData::Vec2(new_enemy_pos.into()));
        }
    }
}

fn vector_between_coordinates(from: Vec2, to: Vec2) -> Vec2 {
    to - from
}

register_zurie_mod!(Game);
