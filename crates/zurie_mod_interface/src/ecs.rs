use crate::engine::ecs::{self, entity_exits, spawn_entity};
use crate::engine::ecs::{ComponentData, despawn_entity};
use crate::engine::sprite;

#[derive(Clone, Copy, Default, Debug)]
pub struct Entity(pub u64);

impl Entity {
    pub fn set_component(self, component: u64, data: ComponentData) -> Self {
        ecs::set_component(self.0, component, &data);
        self
    }

    pub fn set_sprite(self, sprite: u64) -> Self {
        sprite::set_sprite(self.0, sprite);
        self
    }

    pub fn remove_sprite(self) -> Self {
        sprite::remove_sprite(self.0);
        self
    }

    pub fn get_component(self, component: u64) -> Option<ComponentData> {
        ecs::get_component(self.0, component)
    }

    pub fn spawn() -> Self {
        Entity(spawn_entity())
    }

    pub fn despawn(&mut self) {
        despawn_entity(self.0);
    }

    pub fn exits(self) -> bool {
        entity_exits(self.0)
    }
}

pub fn get_entities_with_component(component: u64) -> Vec<Entity> {
    ecs::get_entities_with_component(component)
        .to_vec()
        .iter()
        .map(|ent: &u64| Entity(*ent))
        .collect()
}

pub fn get_entities_with_components(components: &[u64]) -> Vec<Entity> {
    ecs::get_entities_with_components(components)
        .to_vec()
        .iter()
        .map(|ent: &u64| Entity(*ent))
        .collect()
}
