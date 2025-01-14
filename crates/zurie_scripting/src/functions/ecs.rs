use zurie_shared::slotmap::{Key, KeyData};
use zurie_types::ComponentData as EngineComponentData;

use super::ScriptingState;

use crate::functions::zurie::engine::ecs;
use crate::functions::zurie::engine::ecs::*;
impl ecs::Host for ScriptingState {
    fn spawn_entity(&mut self) -> EntityId {
        KeyData::as_ffi(self.world.write().unwrap().spawn_entity().data())
    }

    fn despawn_entity(&mut self, entity: EntityId) -> () {
        self.world
            .write()
            .unwrap()
            .despawn(KeyData::from_ffi(entity).into());
    }

    fn register_component(&mut self, name: String) -> ComponentId {
        KeyData::as_ffi(self.world.write().unwrap().register_component(name).data())
    }

    fn set_component(
        &mut self,
        entity: EntityId,
        component: ComponentId,
        data: ComponentData,
    ) -> () {
        let data: EngineComponentData = data.into();
        let entity = KeyData::from_ffi(entity).into();
        let component = KeyData::from_ffi(component).into();
        self.world
            .write()
            .unwrap()
            .set_component(entity, (component, data));
    }

    fn get_component(&mut self, entity: EntityId, component: ComponentId) -> Option<ComponentData> {
        match self.world.read().unwrap().get_component(
            KeyData::from_ffi(entity).into(),
            KeyData::from_ffi(component).into(),
        ) {
            Some(data) => Some(ComponentData::from(data)),
            None => None,
        }
    }

    fn get_entities_with_component(&mut self, component: ComponentId) -> Vec<EntityId> {
        self.world
            .read()
            .unwrap()
            .get_entities_with_component(KeyData::from_ffi(component).into())
            .iter()
            .map(|component| KeyData::as_ffi(component.data()))
            .collect()
    }

    fn get_entities_with_components(
        &mut self,
        required: Vec<ComponentId>,
        optional: Vec<ComponentId>,
    ) -> wasmtime::component::__internal::Vec<EntityId> {
        self.world
            .read()
            .unwrap()
            .get_entities_with_arhetype(zurie_ecs::Architype {
                required: required
                    .iter()
                    .map(|component| KeyData::from_ffi(*component).into())
                    .collect(),
                optional: optional
                    .iter()
                    .map(|component| KeyData::from_ffi(*component).into())
                    .collect(),
            })
            .iter()
            .map(|component| KeyData::as_ffi(component.data()))
            .collect()
    }
}

impl From<ComponentData> for EngineComponentData {
    fn from(data: ComponentData) -> Self {
        match data {
            ComponentData::None => EngineComponentData::None,
            ComponentData::Str(s) => EngineComponentData::String(s),
            ComponentData::Vec2(v) => EngineComponentData::Vector(v.into()),
            ComponentData::Color(c) => EngineComponentData::Color([c.r, c.g, c.b, c.a]),
            ComponentData::Raw(bytes) => EngineComponentData::Raw(bytes),
            ComponentData::I32(i) => EngineComponentData::I32(i),
            ComponentData::I64(i) => EngineComponentData::I64(i),
            ComponentData::Sprite(sprite_handle) => EngineComponentData::Sprite(sprite_handle),
        }
    }
}

// Convert from Engine types to WIT types
impl From<EngineComponentData> for ComponentData {
    fn from(data: EngineComponentData) -> Self {
        match data {
            EngineComponentData::None => ComponentData::None,
            EngineComponentData::String(s) => ComponentData::Str(s),
            EngineComponentData::Vector(v) => ComponentData::Vec2(v.into()),
            EngineComponentData::Color(c) => ComponentData::Color(Color {
                r: c[0],
                g: c[1],
                b: c[2],
                a: c[3],
            }),
            EngineComponentData::Raw(bytes) => ComponentData::Raw(bytes),
            EngineComponentData::I32(i) => ComponentData::I32(i),
            EngineComponentData::I64(i) => ComponentData::I64(i),
            EngineComponentData::Sprite(sprite_handle) => ComponentData::Sprite(sprite_handle),
        }
    }
}

impl From<&EngineComponentData> for ComponentData {
    fn from(data: &EngineComponentData) -> Self {
        match data {
            EngineComponentData::None => ComponentData::None,
            EngineComponentData::String(s) => ComponentData::Str(s.clone()),
            EngineComponentData::Vector(v) => ComponentData::Vec2((*v).into()),
            EngineComponentData::Color(c) => ComponentData::Color(Color {
                r: c[0],
                g: c[1],
                b: c[2],
                a: c[3],
            }),
            EngineComponentData::Raw(bytes) => ComponentData::Raw(bytes.clone()),
            EngineComponentData::I32(i) => ComponentData::I32(*i),
            EngineComponentData::I64(i) => ComponentData::I64(*i),
            EngineComponentData::Sprite(sprite_handle) => ComponentData::Sprite(*sprite_handle),
        }
    }
}
