use std::iter;
use std::ops::Range;
use zurie_mod_interface::engine::camera::set_zoom;
use zurie_mod_interface::engine::core::{ComponentId, SpriteHandle};
use zurie_mod_interface::engine::input::left_mouse_clicked;
use zurie_mod_interface::engine::sprite::{load_sprite_file, set_sprite};
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
        let player_sprite = load_sprite_file("static/player.aseprite");
        let enemy_sprite = load_sprite_file("static/error.aseprite");
        self.test_sprite = load_sprite_file("static/ase2.aseprite");

        let player_ent = Entity::spawn();
        let pos_component = register_component("position");
        let enemy_component = register_component("enemy");
        player_ent.set_component(
            pos_component,
            zurie_mod_interface::engine::ecs::ComponentData::Vec2(
                zurie_mod_interface::engine::core::Vec2 { x: 0.0, y: 0.0 },
            ),
        );
        self.player = player_ent;
        self.pos_component = pos_component;
        self.player.set_sprite(player_sprite);
        spawn_enemies(
            enemy_component,
            pos_component,
            Vec2::new(0.0, 0.0).into(),
            enemy_sprite,
        );
        set_zoom(15.0);
        info!("Mod inited!!");
    }

    fn key_event(&mut self, key: zurie_mod_interface::input::KeyCode) {
        if key == zurie_mod_interface::input::KeyCode::KeyO {
            play_sound(self.sound);
        }
    }

    fn update(&mut self) {
        let mut direction = Vec2::ZERO;

        if key_clicked(zurie_mod_interface::input::KeyCode::KeyW) {
            direction += Vec2::new(0.0, -1.0)
        }
        if key_clicked(zurie_mod_interface::input::KeyCode::KeyA) {
            direction += Vec2::new(-1.0, 0.0)
        }
        if key_clicked(zurie_mod_interface::input::KeyCode::KeyS) {
            direction += Vec2::new(0.0, 1.0)
        }
        if key_clicked(zurie_mod_interface::input::KeyCode::KeyD) {
            direction += Vec2::new(1.0, 0.0)
        }

        if let Some(ComponentData::Vec2(old_player_pos)) =
            self.player.get_component(self.pos_component)
        {
            let new_pos: Vec2 =
                (<zurie_mod_interface::engine::core::Vec2 as Into<Vec2>>::into(old_player_pos))
                    + direction * 0.1;

            self.player
                .set_component(self.pos_component, ComponentData::Vec2(new_pos.into()));
            // if left_mouse_clicked() {
            //     Entity::spawn()
            //         .set_component(self.pos_component, ComponentData::Vec2(old_player_pos))
            //         .set_sprite(self.test_sprite);
            // }
        }
        let widgets = vec![
            Widget::Label("My custom label in window".into()),
            Widget::Button("My custom button. Try to click me".into()),
        ];
        let responces = create_window("My window", &widgets);
        if let Some(WidgetResponse::Clicked(clicked)) = responces.get(1) {
            if *clicked {
                info!("Mouse clicked")
            }
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
    spawn_enemy(
        enemy_component,
        pos_component,
        Vec2 { x: 1.0, y: 1.0 },
        sprite,
    );
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

register_zurie_mod!(Game);
