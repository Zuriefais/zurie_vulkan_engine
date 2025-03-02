#![feature(extract_if)]

use egui::{Context, Label};
use hashbrown::HashSet;
use log::info;
use serde::{Deserialize, Serialize};
use zurie_shared::slotmap::{Key, KeyData, SlotMap, new_key_type};
use zurie_types::ComponentData;

new_key_type! { pub struct Entity; }
new_key_type! { pub struct ComponentID; }

use std::fmt::Display;
impl Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", KeyData::as_ffi(self.0))
    }
}
impl Display for ComponentID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", KeyData::as_ffi(self.0))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Architype {
    pub required: Vec<ComponentID>,
    pub optional: Vec<ComponentID>,
}

#[derive(Default, Debug, Clone)]
pub struct EntityData {
    pub data: Vec<(ComponentID, ComponentData)>,
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

    pub fn get_entities_data_with_components(
        &self,
        components: Vec<ComponentID>,
    ) -> Vec<(Entity, &EntityData)> {
        let mut entities = Vec::with_capacity(self.entities.len() / 2);

        let required_component_ids: HashSet<ComponentID> = components.iter().copied().collect();

        'entity_loop: for (entity, data) in self.entities.iter() {
            let entity_component_ids: HashSet<ComponentID> =
                data.data.iter().map(|(id, _)| *id).collect();

            // Check if all required components are present
            if !entity_component_ids.is_superset(&required_component_ids) {
                continue 'entity_loop;
            }
            entities.push((entity, data));
        }

        entities
    }

    pub fn get_entities_with_components(&self, components: Vec<ComponentID>) -> Vec<Entity> {
        let mut entities = Vec::with_capacity(self.entities.len() / 2);

        let required_component_ids: HashSet<ComponentID> = components.iter().copied().collect();

        info!(
            "requested find entities with: {:?}",
            &components
                .iter()
                .map(|comp| { KeyData::as_ffi(comp.data()) })
                .collect::<Vec<_>>()
        );

        'entity_loop: for (entity, data) in self.entities.iter() {
            let entity_component_ids: HashSet<ComponentID> =
                data.data.iter().map(|(id, _)| *id).collect();

            if !entity_component_ids.is_superset(&required_component_ids) {
                continue 'entity_loop;
            }
            entities.push(entity);
        }

        entities
    }

    pub fn get_entities_with_component(&self, component: ComponentID) -> Vec<Entity> {
        let mut entities = Vec::with_capacity(self.entities.len() / 2);

        info!(
            "requested find entities with: {:?}",
            KeyData::as_ffi(component.data())
        );

        for (entity, data) in self.entities.iter() {
            // Check if the entity has the component we're looking for
            if data.data.iter().any(|(comp_id, _)| *comp_id == component) {
                entities.push(entity);
            }
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
                    return;
                }
            }
            info!("setting component: {:?}", &new_component);
            entity_data.data.push(new_component);
        }
    }

    pub fn remove_component(&mut self, entity: Entity, component: ComponentID) {
        if let Some(entity_data) = self.entities.get_mut(entity) {
            entity_data
                .data
                .retain(|(ent_component, _)| *ent_component != component);
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

    pub fn get_entities_data_with_components(
        &self,
        components: Vec<ComponentID>,
    ) -> Vec<(Entity, &EntityData)> {
        self.storage.get_entities_data_with_components(components)
    }
    pub fn get_entities_with_components(&self, components: Vec<ComponentID>) -> Vec<Entity> {
        self.storage.get_entities_with_components(components)
    }

    pub fn get_entities_with_component(&self, component: ComponentID) -> Vec<Entity> {
        self.storage.get_entities_with_component(component)
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

    pub fn remove_component(&mut self, entity: Entity, component: ComponentID) {
        self.storage.remove_component(entity, component)
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

    pub fn inspector(&mut self, context: Context) {
        let window = egui::Window::new("Inspector");
        window.show(&context, |ui| {
            // Make components list collapsible
            egui::CollapsingHeader::new("Registered Components")
                .default_open(true) // Optional: starts expanded
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for component in self.registered_components.iter() {
                            ui.add(Label::new(format!(
                                "id: {}, name: {}",
                                component.0, component.1
                            )));
                        }
                    });
                });
            ui.label(format!("Entities count: {}", self.storage.entities.len()));
            egui::CollapsingHeader::new("Entities")
                .default_open(true)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (entity, components) in self.storage.entities.iter() {
                            egui::CollapsingHeader::new(format!("Entity {}", entity))
                                .id_salt(entity.0) // Unique ID for each entity header
                                .show(ui, |ui| {
                                    for (component_id, component) in components.data.iter() {
                                        let component_name =
                                            match self.registered_components.get(*component_id) {
                                                Some(name) => name,
                                                None => "Unknown",
                                            };
                                        let text = match component {
                                            ComponentData::String(s) => format!("String: {}", s),
                                            ComponentData::Vector(v) => format!("Vector: {:?}", v),
                                            ComponentData::Color(c) => format!("Color: {:?}", c),
                                            ComponentData::Raw(r) => format!("Raw: {:?}", r),
                                            ComponentData::None => "None".to_string(),
                                            ComponentData::Sprite(h) => format!("Sprite: {h}"),
                                            ComponentData::I32(i) => format!("I32: {i}"),
                                            ComponentData::I64(i) => format!("I64: {i}"),
                                        };
                                        ui.label(format!(
                                            "Component {} ({}): {}",
                                            component_id, component_name, text
                                        ));
                                    }
                                });
                        }
                    });
                });
        });
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use zurie_types::glam::Vec2;

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
        world.modify_entity(entity, EntityData {
            data: vec![(my_component, ComponentData::Raw(vec![20]))],
        });
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
            world
                .storage
                .get_entities_data_with_components([my_component, my_component3].into(),),
        );
        assert_eq!(
            200,
            world
                .storage
                .get_entities_data_with_components([my_component, my_component3].into(),)
                .len()
        );
    }

    #[test]
    fn test_archetype_order_independence() {
        let mut world = World::default();
        let comp_a = world.register_component("comp_a".into());
        let comp_b = world.register_component("comp_b".into());

        world.spawn_entity_with_data(EntityData {
            data: vec![
                (comp_a, ComponentData::Raw(vec![1])),
                (comp_b, ComponentData::Raw(vec![2])),
            ],
        });

        // Create entity with components in different order
        world.spawn_entity_with_data(EntityData {
            data: vec![
                (comp_b, ComponentData::Raw(vec![2])),
                (comp_a, ComponentData::Raw(vec![1])),
            ],
        });

        let matches = world.get_entities_data_with_components([comp_a, comp_b].into());

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_entity_creation() {
        let mut world = World::default();
        let entity = world.spawn_entity();
        assert!(world.get_entity_data(entity).is_some());
    }

    #[test]
    fn test_component_registration() {
        let mut world = World::default();
        let component_id = world.register_component("Health".to_string());
        assert!(world.registered_components.get(component_id).is_some());
    }

    #[test]
    fn test_set_component() {
        let mut world = World::default();
        let entity = world.spawn_entity();
        let component_id = world.register_component("Health".to_string());
        let component_data = ComponentData::I32(100);

        world.set_component(entity, (component_id, component_data.clone()));
        let stored_data = world.get_component(entity, component_id);

        assert!(stored_data.is_some());
        assert_eq!(stored_data.unwrap(), &component_data);
    }

    #[test]
    fn test_remove_component() {
        let mut world = World::default();
        let entity = world.spawn_entity();
        let component_id = world.register_component("Health".to_string());
        let component_data = ComponentData::I32(100);

        world.set_component(entity, (component_id, component_data));
        world.remove_component(entity, component_id);

        assert!(world.get_component(entity, component_id).is_none());
    }

    #[test]
    fn test_entity_with_component_query() {
        let mut world = World::default();
        let entity = world.spawn_entity();
        let component_id = world.register_component("Health".to_string());
        let component_data = ComponentData::I32(100);

        world.set_component(entity, (component_id, component_data));
        let entities = world.get_entities_with_component(component_id);

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0], entity);
    }

    #[test]
    fn test_get_entities_with_multiple_components() {
        let mut world = World::default();
        let entity = world.spawn_entity();
        let health_id = world.register_component("Health".to_string());
        let position_id = world.register_component("Position".to_string());

        world.set_component(entity, (health_id, ComponentData::I32(100)));
        world.set_component(
            entity,
            (position_id, ComponentData::Vector(Vec2::new(1.0, 1.0))),
        );

        let query_components = [health_id, position_id];
        let entities = world.get_entities_with_components(query_components.into());

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0], entity);
    }

    #[test]
    fn test_despawn_entity() {
        let mut world = World::default();
        let entity = world.spawn_entity();
        world.despawn(entity);

        assert!(world.get_entity_data(entity).is_none());
    }
}
