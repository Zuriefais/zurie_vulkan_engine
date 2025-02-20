use log::info;
use zurie_event::EventData as EngineEventData;

use super::{ScriptingState, zurie::engine::events};
use crate::functions::zurie::engine::events::*;
use zurie_shared::slotmap::{Key, KeyData};

impl events::Host for ScriptingState {
    fn subscribe_by_name(&mut self, name: String) -> events::EventHandle {
        KeyData::as_ffi(
            self.event_manager
                .write()
                .unwrap()
                .subscribe_by_name(name, self.mod_handle)
                .data(),
        )
    }

    fn subscribe_by_handle(&mut self, handle: events::EventHandle) -> () {
        self.event_manager
            .write()
            .unwrap()
            .subscribe_by_handle(KeyData::from_ffi(handle).into(), self.mod_handle);
    }

    fn emit(&mut self, handle: events::EventHandle, data: EventData) -> () {
        let mut event_manager = self.event_manager.write().unwrap();
        let handle = KeyData::from_ffi(handle);
        info!(
            "Event emited: {}",
            event_manager.event_storage.get(handle.into()).unwrap()
        );
        event_manager.emit(&self.mod_handle, zurie_event::Event {
            handle: handle.into(),
            data: data.into(),
        });
    }
}

impl From<EventData> for EngineEventData {
    fn from(data: EventData) -> Self {
        match data {
            EventData::None => EngineEventData::None,
            EventData::Str(s) => EngineEventData::String(s),
            EventData::Vec2(v) => EngineEventData::Vector(v.into()),
            EventData::Color(c) => EngineEventData::Color([c.r, c.g, c.b, c.a]),
            EventData::Raw(bytes) => EngineEventData::Raw(bytes),
            EventData::I32(i) => EngineEventData::I32(i),
            EventData::I64(i) => EngineEventData::I64(i),
        }
    }
}

// Convert from Engine types to WIT types
impl From<EngineEventData> for EventData {
    fn from(data: EngineEventData) -> Self {
        match data {
            EngineEventData::None => EventData::None,
            EngineEventData::String(s) => EventData::Str(s),
            EngineEventData::Vector(v) => EventData::Vec2(v.into()),
            EngineEventData::Color(c) => EventData::Color(Color {
                r: c[0],
                g: c[1],
                b: c[2],
                a: c[3],
            }),
            EngineEventData::Raw(bytes) => EventData::Raw(bytes),
            EngineEventData::I32(i) => EventData::I32(i),
            EngineEventData::I64(i) => EventData::I64(i),
        }
    }
}

// Vec2 conversions
impl From<zurie_types::glam::Vec2> for Vec2 {
    fn from(v: zurie_types::glam::Vec2) -> Self {
        Vec2 { x: v.x, y: v.y }
    }
}

impl From<Vec2> for zurie_types::glam::Vec2 {
    fn from(v: Vec2) -> Self {
        zurie_types::glam::Vec2 { x: v.x, y: v.y }
    }
}
