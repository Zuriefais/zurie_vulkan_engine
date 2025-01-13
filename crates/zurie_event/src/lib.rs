use hashbrown::{HashMap, HashSet};
use log::info;
use slotmap::{Key, KeyData, SlotMap, new_key_type};
use std::sync::{Arc, RwLock};
use zurie_types::{ModHandle, glam::Vec2};
new_key_type! { pub struct EventHandle; }

#[derive(Clone)]
pub struct Event {
    pub handle: EventHandle,
    pub data: EventData,
}

#[derive(Clone, Default)]
pub struct EventManager {
    pub event_storage: SlotMap<EventHandle, String>,
    pub event_handlers: HashMap<ModHandle, HashSet<EventHandle>>,
    pub event_queue: HashMap<ModHandle, ModEventQueue>,
}

#[derive(Clone, Default)]
pub struct ModEventQueue {
    store: Arc<RwLock<Vec<Event>>>,
}

impl ModEventQueue {
    pub fn join(&mut self, event: Event) {
        self.store.write().unwrap().push(event);
    }

    pub fn drain(&mut self) -> Vec<Event> {
        let mut store = self.store.write().unwrap();
        std::mem::take(&mut *store) // Take ownership of the current vector and replace it with empty
    }
}

impl Iterator for ModEventQueue {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        let mut store = self.store.write().unwrap();
        store.pop()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventData {
    I32(i32),
    I64(i64),
    String(String),
    Vector(Vec2),
    Color([f32; 4]),
    Raw(Vec<u8>),
    None,
}

impl EventManager {
    pub fn subscribe_by_handle(&mut self, event_handle: EventHandle, mod_handle: ModHandle) {
        match self.event_handlers.get_mut(&mod_handle) {
            Some(subscribed) => {
                subscribed.insert(event_handle);
            }
            None => {
                self.event_handlers.insert(mod_handle, {
                    let mut subscribed = HashSet::new();
                    subscribed.insert(event_handle);
                    subscribed
                });
            }
        }
    }
    pub fn subscribe_by_name(&mut self, name: String, mod_handle: ModHandle) -> EventHandle {
        let event_handle = self
            .event_storage
            .iter()
            .find(|(_, event_name)| name == **event_name)
            .map(|(key, _)| key)
            .unwrap_or_else(|| self.event_storage.insert(name.clone()));
        self.subscribe_by_handle(event_handle, mod_handle);
        info!("Event registered: {}", name);
        event_handle
    }

    pub fn emit(&mut self, mod_handle: &ModHandle, event: Event) {
        for (subscriber_handle, subscribed_events) in self.event_handlers.iter() {
            {
                if subscribed_events.contains(&event.handle) {
                    if subscriber_handle != mod_handle {
                        if let Some(queue) = self.event_queue.get_mut(subscriber_handle) {
                            queue.join(event.clone());
                        }
                    }
                }
            }
        }
    }

    pub fn mod_subscribe(&mut self, queue: ModEventQueue, handle: ModHandle) {
        self.event_queue.insert(handle, queue);
    }
}
