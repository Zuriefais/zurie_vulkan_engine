use zurie_shared::slotmap::{new_key_type, SlotMap};

new_key_type! { pub struct Entity; }
new_key_type! { pub struct ComponentID; }

pub struct Architype {
    pub data: Vec<ComponentID>,
}

#[derive(Default)]
pub struct EntityData {
    pub data: Vec<(ComponentID, Vec<u8>)>,
}

#[derive(Default)]
pub struct EntityStorage {
    entities: SlotMap<Entity, EntityData>,
}

impl EntityStorage {
    pub fn spawn_entity(&mut self) -> Entity {
        self.entities.insert(EntityData::default())
    }

    pub fn spawn_entity_with_data(&mut self, data: EntityData) -> Entity {
        self.entities.insert(data)
    }

    pub fn get_entity_data(&self, entity: Entity) -> Option<&EntityData> {
        self.entities.get(entity)
    }

    pub fn get_all_entities(&self) -> Vec<(Entity, &EntityData)> {
        self.entities.iter().collect()
    }

    pub fn get_entities_with_arhetype(&self, architype: Architype) -> Vec<(Entity, &EntityData)> {
        let mut entities = vec![];
        'entity_loop: for (entity, data) in self.entities.iter() {
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

    pub fn despawn(&mut self, entity: Entity) {
        self.entities.remove(entity);
    }
}

#[derive(Default)]
pub struct World {
    pub storage: EntityStorage,
    pub registered_components: SlotMap<ComponentID, String>,
}

impl World {
    pub fn register_component(&mut self, name: String) -> ComponentID {
        self.registered_components.insert(name)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_ecs() {
        let mut world = World::default();
        let my_component = world.register_component("my_component".into());
        let entity = world.storage.spawn_entity_with_data(EntityData {
            data: vec![(my_component, vec![10])],
        });
        assert_eq!(
            vec![10],
            world.storage.get_entity_data(entity).unwrap().data[0].1
        );
    }
}
