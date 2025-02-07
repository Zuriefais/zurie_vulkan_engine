use std::time::{Duration, Instant};
use zurie_mod_interface::ecs::get_entities_with_component;
use zurie_mod_interface::engine;
use zurie_mod_interface::engine::camera::set_zoom;
use zurie_mod_interface::engine::core::{ComponentId, SpriteHandle};

use zurie_mod_interface::ecs::get_entities_with_components;
use zurie_mod_interface::engine::input::key_clicked;
use zurie_mod_interface::engine::sprite::load_sprite_bin;
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
    log::info,
    register_zurie_mod,
};

pub struct Game {
    sound: u64,
    player: Entity,
    pos_component: u64,
    enemy_component: u64,
    enemy_sprite: u64,
    projectile_component: u64,
    health_component: u64,
    last_shot: Instant,
    projectile_sprite: u64,
    direction_component: u64,
    next_enemy_wave: Instant,
    player_sprite: u64,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            sound: 0,
            player: Entity::default(),
            pos_component: 0,
            enemy_component: 0,
            projectile_component: 0,
            health_component: 0,
            enemy_sprite: 0,
            last_shot: Instant::now(),
            projectile_sprite: 0,
            direction_component: 0,
            next_enemy_wave: Instant::now(),
            player_sprite: 0,
        }
    }
}

impl ZurieMod for Game {
    fn new() -> Self {
        Game::default()
    }

    fn get_mod_name(&self) -> String {
        "Vampire Survivors Prototype".into()
    }

    fn init(&mut self) {
        self.sound = load_sound("static/sound.wav");
        let player_sprite = load_sprite_bin(include_bytes!("../../../static/player.aseprite"));
        let enemy_sprite = load_sprite_bin(include_bytes!("../../../static/enemy.aseprite"));
        self.projectile_sprite =
            load_sprite_bin(include_bytes!("../../../static/projectile.aseprite"));

        let player_ent = Entity::spawn();
        let pos_component = register_component("position");
        let enemy_component = register_component("enemy");
        let projectile_component = register_component("projectile");
        let health_component = register_component("health");
        let direction_component = register_component("direction");

        player_ent.set_component(pos_component, ComponentData::Vec2(Vec2::ZERO.into()));
        player_ent.set_component(health_component, ComponentData::I32(100));
        player_ent.set_sprite(player_sprite);

        self.player = player_ent;
        self.player_sprite = player_sprite;
        self.pos_component = pos_component;
        self.enemy_component = enemy_component;
        self.projectile_component = projectile_component;
        self.health_component = health_component;
        self.last_shot = Instant::now();
        self.direction_component = direction_component;
        self.enemy_sprite = enemy_sprite;

        spawn_enemy_wave(
            enemy_component,
            pos_component,
            health_component,
            enemy_sprite,
        );

        set_zoom(10.0);
    }

    fn update(&mut self) {
        if !self.player.exits() {
            let responses = create_window("Game Status", &[
                Widget::Label("You Lose!!!".into()),
                Widget::Button("Restart Game?".into()),
            ]);
            if let WidgetResponse::Clicked(clicked) = responses[1] {
                if clicked {
                    for enemy in get_entities_with_component(self.enemy_component).iter_mut() {
                        enemy.despawn();
                    }

                    let player_ent = Entity::spawn()
                        .set_component(self.pos_component, ComponentData::Vec2(Vec2::ZERO.into()))
                        .set_component(self.health_component, ComponentData::I32(100))
                        .set_sprite(self.player_sprite);
                    self.player = player_ent
                }
            }
            return;
        }

        let direction = Vec2::new(
            (key_clicked(zurie_mod_interface::input::KeyCode::KeyD as u32) as i8
                - key_clicked(zurie_mod_interface::input::KeyCode::KeyA as u32) as i8)
                as f32,
            (key_clicked(zurie_mod_interface::input::KeyCode::KeyS as u32) as i8
                - key_clicked(zurie_mod_interface::input::KeyCode::KeyW as u32) as i8)
                as f32,
        );

        if let Some(ComponentData::Vec2(old_pos)) = self.player.get_component(self.pos_component) {
            let new_pos = Into::<Vec2>::into(old_pos) + direction * 0.5;
            self.player
                .set_component(self.pos_component, ComponentData::Vec2(new_pos.into()));
        }

        move_enemies(self.pos_component, self.enemy_component, self.player);

        if self.last_shot.elapsed() > Duration::from_secs_f32(0.5) {
            fire_projectile(
                self.player,
                self.pos_component,
                self.projectile_component,
                self.projectile_sprite,
                self.enemy_component,
                self.direction_component,
            );
            self.last_shot = Instant::now();
        }

        if self.next_enemy_wave.elapsed() > Duration::from_secs_f32(5.0) {
            spawn_enemy_wave(
                self.enemy_component,
                self.pos_component,
                self.health_component,
                self.enemy_sprite,
            );
            self.next_enemy_wave = Instant::now();
        }

        update_projectiles(
            self.projectile_component,
            self.pos_component,
            self.direction_component,
        );

        check_projectile_collision(
            self.projectile_component,
            self.pos_component,
            self.direction_component,
            self.enemy_component,
            self.health_component,
        );

        check_player_collision(
            self.player,
            self.pos_component,
            self.enemy_component,
            self.health_component,
        );
    }
}

fn spawn_enemy_wave(
    enemy_component: ComponentId,
    pos_component: ComponentId,
    health_component: ComponentId,
    sprite: SpriteHandle,
) {
    for i in -2..=2 {
        let enemy_pos = Vec2::new(i as f32 * 2.0, -5.0);
        spawn_enemy(
            enemy_component,
            pos_component,
            health_component,
            enemy_pos,
            sprite,
        );
    }
}

fn spawn_enemy(
    enemy_component: ComponentId,
    pos_component: ComponentId,
    health_component: ComponentId,
    pos: Vec2,
    sprite: SpriteHandle,
) {
    Entity::spawn()
        .set_component(pos_component, ComponentData::Vec2(pos.into()))
        .set_component(health_component, ComponentData::I32(100))
        .set_component(enemy_component, ComponentData::None)
        .set_sprite(sprite);
}

fn move_enemies(pos_component: ComponentId, enemy_component: ComponentId, player: Entity) {
    let enemies = get_entities_with_component(enemy_component);
    if let Some(ComponentData::Vec2(player_pos)) = player.get_component(pos_component) {
        for enemy in enemies.iter() {
            if let Some(ComponentData::Vec2(enemy_pos)) = enemy.get_component(pos_component) {
                let new_pos: Vec2 = Into::<Vec2>::into(enemy_pos)
                    + (Into::<Vec2>::into(player_pos) - Into::<Vec2>::into(enemy_pos))
                        .normalize_or_zero()
                        * 0.05;
                enemy.set_component(pos_component, ComponentData::Vec2(new_pos.into()));
            }
        }
    }
}

fn fire_projectile(
    player: Entity,
    pos_component: ComponentId,
    projectile_component: ComponentId,
    projectile_sprite: u64,
    enemy_component: ComponentId,
    direction_component: ComponentId,
) {
    if let Some(ComponentData::Vec2(player_pos)) = player.get_component(pos_component) {
        let nearest_enemy_pos: Option<Vec2> =
            get_nearest_enemy_to(pos_component, enemy_component, player_pos.into());
        if let Some(enemy_pos) = nearest_enemy_pos {
            Entity::spawn()
                .set_component(pos_component, ComponentData::Vec2(player_pos.into()))
                .set_component(
                    direction_component,
                    ComponentData::Vec2(
                        vector_between_coordinates(player_pos.into(), enemy_pos)
                            .normalize()
                            .into(),
                    ),
                )
                .set_component(projectile_component, ComponentData::None)
                .set_sprite(projectile_sprite);
        }
    }
}

fn update_projectiles(
    projectile_component: ComponentId,
    pos_component: ComponentId,
    direction_component: ComponentId,
) {
    let projectiles = get_entities_with_component(projectile_component);
    for projectile in projectiles.iter() {
        if let (Some(ComponentData::Vec2(proj_pos)), Some(ComponentData::Vec2(proj_dir))) = (
            projectile.get_component(pos_component),
            projectile.get_component(direction_component),
        ) {
            let new_pos: Vec2 = Into::<Vec2>::into(proj_pos) + Into::<Vec2>::into(proj_dir) * 0.1;
            projectile.set_component(pos_component, ComponentData::Vec2(new_pos.into()));
        }
    }
}

fn check_projectile_collision(
    projectile_component: ComponentId,
    pos_component: ComponentId,
    direction_component: ComponentId,
    enemy_component: ComponentId,
    health_component: ComponentId,
) {
    let mut projectiles = get_entities_with_component(projectile_component);
    let mut enemies = get_entities_with_component(enemy_component);

    projectiles.iter_mut().for_each(|projectile| {
        if let Some(ComponentData::Vec2(proj_pos)) = projectile.get_component(pos_component) {
            let proj_pos: Vec2 = proj_pos.into();
            let mut collided = false;
            enemies.iter_mut().for_each(|enemy| {
                if let Some(ComponentData::Vec2(enemy_pos)) = enemy.get_component(pos_component) {
                    let enemy_pos: Vec2 = enemy_pos.into();

                    if proj_pos.distance(enemy_pos) < 0.5 {
                        collided = true;
                        enemy.get_component(health_component).map(|health| {
                            if let ComponentData::I32(health_value) = health {
                                let new_health = health_value - 10;
                                enemy.set_component(
                                    health_component,
                                    ComponentData::I32(new_health),
                                );

                                if new_health <= 0 {
                                    enemy.despawn();
                                }
                            }
                        });
                    }
                }
            });
            if collided {
                projectile.despawn();
            }
        }
    });
}

fn check_player_collision(
    mut player: Entity,
    pos_component: ComponentId,
    enemy_component: ComponentId,
    health_component: ComponentId,
) {
    if let (Some(ComponentData::Vec2(pos)), Some(ComponentData::I32(health))) = (
        player.get_component(pos_component),
        player.get_component(health_component),
    ) {
        let enemies = get_enemies_with_distance_to(pos_component, enemy_component, pos.into(), 1.0)
            .len() as i32;
        let new_health = health - 5 * enemies;
        if health - 5 * enemies < 0 {
            player.despawn()
        } else {
            player.set_component(health_component, ComponentData::I32(health - 5 * enemies));
        }
    }
}

fn get_enemies_with_distance_to(
    pos_component: ComponentId,
    enemy_component: ComponentId,
    to: Vec2,
    distance: f32,
) -> Vec<Entity> {
    get_entities_with_components(&[pos_component, enemy_component])
        .iter()
        .filter(|ent| {
            if let ComponentData::Vec2(pos) = ent
                .get_component(pos_component)
                .unwrap_or(engine::ecs::ComponentData::Vec2(Vec2::ZERO.into()))
            {
                if Into::<Vec2>::into(pos).distance(to) < distance {
                    true
                } else {
                    false
                }
            } else {
                false
            }
        })
        .copied()
        .collect()
}

fn get_nearest_enemy_to(
    pos_component: ComponentId,
    enemy_component: ComponentId,
    to: Vec2,
) -> Option<Vec2> {
    get_entities_with_components(&[pos_component, enemy_component])
        .iter()
        .map(|ent| {
            if let ComponentData::Vec2(pos) = ent
                .get_component(pos_component)
                .unwrap_or(engine::ecs::ComponentData::Vec2(Vec2::ZERO.into()))
            {
                pos.into()
            } else {
                Vec2::ZERO
            }
        })
        .min_by(|a, b| {
            a.distance(to)
                .partial_cmp(&b.distance(to))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn vector_between_coordinates(from: Vec2, to: Vec2) -> Vec2 {
    to - from
}

register_zurie_mod!(Game);
