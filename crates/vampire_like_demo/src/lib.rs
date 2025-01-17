use zurie_mod_interface::engine::camera::set_zoom;
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

        let player_ent = Entity::spawn();
        let pos_component = register_component("position");
        player_ent.set_component(
            pos_component,
            zurie_mod_interface::engine::ecs::ComponentData::Vec2(
                zurie_mod_interface::engine::core::Vec2 { x: 0.0, y: 0.0 },
            ),
        );
        self.player = player_ent;
        self.pos_component = pos_component;
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
            direction += Vec2::new(0.0, 1.0)
        }
        if key_clicked(zurie_mod_interface::input::KeyCode::KeyA) {
            direction += Vec2::new(-1.0, 0.0)
        }
        if key_clicked(zurie_mod_interface::input::KeyCode::KeyS) {
            direction += Vec2::new(0.0, -1.0)
        }
        if key_clicked(zurie_mod_interface::input::KeyCode::KeyD) {
            direction += Vec2::new(1.0, 0.0)
        }
        info!("New direction: {:?}", direction);

        if let Some(ComponentData::Vec2(old_player_pos)) =
            self.player.get_component(self.pos_component)
        {
            let new_pos: Vec2 =
                (<zurie_mod_interface::engine::core::Vec2 as Into<Vec2>>::into(old_player_pos))
                    + direction * 0.1;

            self.player
                .set_component(self.pos_component, ComponentData::Vec2(new_pos.into()));
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

register_zurie_mod!(Game);
