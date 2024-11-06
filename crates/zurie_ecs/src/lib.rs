use log::info;
use serde::{Deserialize, Serialize};
use zurie_shared::slotmap::{new_key_type, SlotMap};
use zurie_types::Vector2;

new_key_type! { pub struct Entity; }
new_key_type! { pub struct ComponentID; }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComponentData {
    String(String),
    Vector(Vector2),
    Color([f32; 4]),
    Scale([f32; 2]),
    Raw(Vec<u8>),
}

impl From<String> for ComponentData {
    fn from(value: String) -> Self {
        ComponentData::String(value)
    }
}

impl From<Vector2> for ComponentData {
    fn from(value: Vector2) -> Self {
        ComponentData::Vector(value)
    }
}

impl From<[f32; 4]> for ComponentData {
    fn from(value: [f32; 4]) -> Self {
        ComponentData::Color(value)
    }
}

impl From<[f32; 2]> for ComponentData {
    fn from(value: [f32; 2]) -> Self {
        ComponentData::Scale(value)
    }
}

impl From<Vec<u8>> for ComponentData {
    fn from(value: Vec<u8>) -> Self {
        ComponentData::Raw(value)
    }
}

impl Default for ComponentData {
    fn default() -> Self {
        ComponentData::String(String::default())
    }
}

pub struct Architype {
    pub data: Vec<ComponentID>,
}

#[derive(Default, Debug)]
pub struct EntityData {
    pub data: Vec<(ComponentID, ComponentData)>,
}

impl Iterator for EntityData {
    type Item = (ComponentID, Vec<u8>);
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

#[derive(Default)]
pub struct EntityStorage {
    entities: SlotMap<Entity, EntityData>,
}

impl EntityStorage {
    pub fn spawn_entity(&mut self) -> Entity {
        self.spawn_entity_with_data(EntityData::default())
    }

    pub fn spawn_entity_with_data(&mut self, data: EntityData) -> Entity {
        info!(
            "Ent spawned. Ent count: {}, component_count, {}",
            self.entities.len(),
            data.data.len()
        );
        self.entities.insert(data)
    }

    pub fn get_entity_data(&self, entity: Entity) -> Option<&EntityData> {
        self.entities.get(entity)
    }

    pub fn get_entity_data_mut(&mut self, entity: Entity) -> Option<&mut EntityData> {
        self.entities.get_mut(entity)
    }

    pub fn get_all_entities(&self) -> Vec<(Entity, &EntityData)> {
        self.entities.iter().collect()
    }

    pub fn get_entities_with_arhetype(&self, architype: Architype) -> Vec<(Entity, &EntityData)> {
        let mut entities = vec![];
        'entity_loop: for (entity, data) in self.entities.iter() {
            if data.data.len() != architype.data.len() {
                continue 'entity_loop;
            }
            for component in architype.data.iter() {
                if !data.data.iter().any(|(id, _)| id == component) {
                    continue 'entity_loop;
                }
            }
            entities.push((entity, data));
        }
        entities
    }

    pub fn modify_entity(&mut self, entity: Entity, new_data: EntityData) {
        if let Some(data) = self.entities.get_mut(entity) {
            *data = new_data
        }
    }

    pub fn set_component(&mut self, entity: Entity, new_component: (ComponentID, ComponentData)) {
        if let Some(entity_data) = self.entities.get_mut(entity) {
            for (component, data) in entity_data.data.iter_mut() {
                if *component == new_component.0 {
                    *data = new_component.1.clone();
                }
            }
        }
    }

    pub fn get_component(
        &self,
        entity: Entity,
        requested_component: ComponentID,
    ) -> Option<&ComponentData> {
        if let Some(data) = self.get_entity_data(entity) {
            for (component, comp_data) in data.data.iter() {
                if *component == requested_component {
                    return Some(comp_data);
                }
            }
        }

        None
    }

    pub fn get_component_mut(
        &mut self,
        entity: Entity,
        requested_component: ComponentID,
    ) -> Option<&mut ComponentData> {
        if let Some(data) = self.get_entity_data_mut(entity) {
            for (component, comp_data) in data.data.iter_mut() {
                if *component == requested_component {
                    return Some(comp_data);
                }
            }
        }

        None
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.entities.remove(entity);
    }
}

#[derive(Default)]
pub struct World {
    storage: EntityStorage,
    registered_components: SlotMap<ComponentID, String>,
}

impl World {
    pub fn register_component(&mut self, name: String) -> ComponentID {
        for component in self.registered_components.iter() {
            if name == *component.1 {
                return component.0;
            }
        }
        self.registered_components.insert(name)
    }

    pub fn spawn_entity(&mut self) -> Entity {
        self.storage.spawn_entity()
    }

    pub fn spawn_entity_with_data(&mut self, data: EntityData) -> Entity {
        self.storage.spawn_entity_with_data(data)
    }

    pub fn get_entity_data(&self, entity: Entity) -> Option<&EntityData> {
        self.storage.get_entity_data(entity)
    }

    pub fn get_all_entities(&self) -> Vec<(Entity, &EntityData)> {
        self.storage.get_all_entities()
    }

    pub fn get_entities_with_arhetype(&self, architype: Architype) -> Vec<(Entity, &EntityData)> {
        self.storage.get_entities_with_arhetype(architype)
    }

    pub fn modify_entity(&mut self, entity: Entity, new_data: EntityData) {
        self.storage.modify_entity(entity, new_data);
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.storage.despawn(entity);
    }

    pub fn set_component(&mut self, entity: Entity, new_component: (ComponentID, ComponentData)) {
        self.storage.set_component(entity, new_component)
    }

    pub fn get_component(&self, entity: Entity, component: ComponentID) -> Option<&ComponentData> {
        self.storage.get_component(entity, component)
    }
    pub fn get_component_mut(
        &mut self,
        entity: Entity,
        component: ComponentID,
    ) -> Option<&mut ComponentData> {
        self.storage.get_component_mut(entity, component)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_one_entity() {
        let mut world = World::default();
        let my_component = world.register_component("my_component".into());
        let entity = world.spawn_entity_with_data(EntityData {
            data: vec![(my_component, ComponentData::Raw(vec![10]))],
        });
        assert_eq!(
            ComponentData::Raw(vec![10]),
            world.get_entity_data(entity).unwrap().data[0].1
        );
        world.modify_entity(
            entity,
            EntityData {
                data: vec![(my_component, ComponentData::Raw(vec![20]))],
            },
        );
        assert_eq!(
            ComponentData::Raw(vec![20]),
            world.get_entity_data(entity).unwrap().data[0].1
        );
    }

    #[test]
    fn test_multiple_entities() {
        let mut world = World::default();
        let my_component = world.register_component("my_component".into());
        let my_component2 = world.register_component("my_component1".into());
        let my_component3 = world.register_component("my_component2".into());

        for num in 0..100 {
            world.storage.spawn_entity_with_data(EntityData {
                data: vec![(my_component, ComponentData::Raw(vec![num]))],
            });
        }
        for num in 0..100 {
            world.storage.spawn_entity_with_data(EntityData {
                data: vec![
                    (my_component, ComponentData::Raw(vec![num])),
                    (my_component2, ComponentData::Raw(vec![num])),
                    (my_component3, ComponentData::Raw(vec![num])),
                ],
            });
        }
        for num in 0..100 {
            world.storage.spawn_entity_with_data(EntityData {
                data: vec![
                    (my_component, ComponentData::Raw(vec![num])),
                    (my_component3, ComponentData::Raw(vec![num])),
                ],
            });
        }
        println!(
            "{:?}",
            world.storage.get_entities_with_arhetype(Architype {
                data: vec![my_component, my_component3]
            })
        );
        assert_eq!(
            100,
            world
                .storage
                .get_entities_with_arhetype(Architype {
                    data: vec![my_component, my_component3]
                })
                .len()
        );
    }
}
