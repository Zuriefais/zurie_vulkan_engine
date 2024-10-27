use zurie_types::serde::Serialize;

use crate::utils::{obj_to_pointer, string_to_pointer};

pub trait EventData {
    fn from_bytes(bytes: &[u8]) -> Self;
    fn to_bytes(&self) -> Vec<u8>;
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct EventHandle {
    id: u64,
}

impl EventHandle {
    pub fn new(id: u64) -> Self {
        Self { id }
    }
}

pub fn subscribe_to_event_by_name(name: &str) -> EventHandle {
    let (ptr, len) = string_to_pointer(name.into());
    EventHandle {
        id: unsafe { subscribe_to_event_by_name_sys(ptr, len) },
    }
}

pub fn subscribe_to_event_by_handle(handle: EventHandle) {
    unsafe { subscribe_to_event_by_handle_sys(handle.id) }
}

pub fn emit_event_(handle: EventHandle, data: &[u8]) {
    unsafe {
        emit_event_sys(handle.id, data.as_ptr() as u32, data.len() as u32);
    }
}

pub fn emit_event_string(handle: EventHandle, data: String) {
    let (ptr, len) = string_to_pointer(data);
    unsafe {
        emit_event_sys(handle.id, ptr, len);
    }
}

pub fn emit_event_obj(handle: EventHandle, data: &impl Serialize) {
    let (ptr, len) = obj_to_pointer(data);
    unsafe {
        emit_event_sys(handle.id, ptr, len);
    }
}

extern "C" {
    fn subscribe_to_event_by_name_sys(ptr: u32, len: u32) -> u64;
    fn subscribe_to_event_by_handle_sys(handle: u64);
    fn emit_event_sys(handle: u64, ptr: u32, len: u32);
}
