use crate::utils::{
    get_bytes_from_mem, get_obj_from_mem, get_string_from_mem, obj_to_pointer, string_to_pointer,
};
use zurie_types::Query;
use zurie_types::{
    glam::Vec2,
    serde::{Deserialize, Serialize},
    ComponentData,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity(u64);

impl Entity {
    pub fn despawn(self) {
        despawn_entity(self);
    }
    pub fn set_component(self, component: ComponentID, data: ComponentData) {
        set_component(self, component, data);
    }
    pub fn get_component_string(self, component: ComponentID) -> Option<String> {
        get_component_string(self, component)
    }
    pub fn get_component_raw(self, component: ComponentID) -> Option<Vec<u8>> {
        get_component_raw(self, component)
    }
    pub fn get_component_scale(self, component: ComponentID) -> Option<[f32; 2]> {
        get_component_scale(self, component)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentID(u64);

pub struct Architype(pub Vec<ComponentID>);

pub fn spawn_entity() -> Entity {
    unsafe { Entity(spawn_entity_sys()) }
}

pub fn despawn_entity(entity: Entity) {
    unsafe { despawn_entity_sys(entity.0) }
}

pub fn register_component(name: &str) -> ComponentID {
    let (ptr, len) = string_to_pointer(name.to_string());
    unsafe { ComponentID(register_component_sys(ptr, len)) }
}

pub fn set_component(entity: Entity, component: ComponentID, data: ComponentData) {
    match data {
        ComponentData::String(str) => {
            let (data_ptr, data_len) = string_to_pointer(str);
            unsafe {
                set_component_string_sys(entity.0, component.0, data_ptr, data_len);
            }
        }
        ComponentData::Vector(vector2) => {
            let (data_ptr, data_len) = obj_to_pointer(&vector2);
            unsafe {
                set_component_vec_sys(entity.0, component.0, data_ptr, data_len);
            }
        }
        ComponentData::Color(color) => {
            let (data_ptr, data_len) = obj_to_pointer(&color);
            unsafe {
                set_component_color_sys(entity.0, component.0, data_ptr, data_len);
            }
        }

        ComponentData::Raw(raw) => {
            let (data_ptr, data_len) = (raw.as_ptr() as u32, raw.len() as u32);
            std::mem::forget(raw);
            unsafe {
                set_component_raw_sys(entity.0, component.0, data_ptr, data_len);
            }
        }
        ComponentData::None => unsafe {
            set_component_none_sys(entity.0, component.0);
        },
        ComponentData::Sprite(handle) => unsafe {
            set_component_sprite_sys(entity.0, handle);
        },
    }
}

pub fn get_component_string(entity: Entity, component: ComponentID) -> Option<String> {
    unsafe {
        if get_component_string_sys(entity.0, component.0) == 1 {
            Some(get_string_from_mem())
        } else {
            None
        }
    }
}

pub fn get_component_raw(entity: Entity, component: ComponentID) -> Option<Vec<u8>> {
    unsafe {
        if get_component_raw_sys(entity.0, component.0) == 1 {
            Some(get_bytes_from_mem())
        } else {
            None
        }
    }
}

pub fn get_component_color(entity: Entity, component: ComponentID) -> Option<[f32; 4]> {
    unsafe {
        if get_component_obj_sys(entity.0, component.0) == 1 {
            Some(get_obj_from_mem())
        } else {
            None
        }
    }
}
pub fn get_component_scale(entity: Entity, component: ComponentID) -> Option<[f32; 2]> {
    unsafe {
        if get_component_obj_sys(entity.0, component.0) == 1 {
            Some(get_obj_from_mem())
        } else {
            None
        }
    }
}
pub fn get_component_vec2(entity: Entity, component: ComponentID) -> Option<Vec2> {
    unsafe {
        if get_component_obj_sys(entity.0, component.0) == 1 {
            Some(get_obj_from_mem())
        } else {
            None
        }
    }
}

pub fn get_entities_with_architype(architype: Vec<ComponentID>) -> Vec<u64> {
    let (ptr, len) = obj_to_pointer(&architype);
    unsafe {
        get_entities_with_architype_sys(ptr, len);
        get_obj_from_mem()
    }
}

pub fn register_query(query: Query) {
    let (data_ptr, data_len) = obj_to_pointer(&query);
    unsafe {
        register_query_sys(data_ptr, data_len);
    }
}

extern "C" {
    fn spawn_entity_sys() -> u64;
    fn despawn_entity_sys(entity_id: u64);
    fn register_component_sys(name_ptr: u32, name_len: u32) -> u64;
    fn set_component_raw_sys(entity_id: u64, component_id: u64, data_ptr: u32, data_len: u32);
    fn set_component_string_sys(entity_id: u64, component_id: u64, data_ptr: u32, data_len: u32);
    fn set_component_vec_sys(entity_id: u64, component_id: u64, data_ptr: u32, data_len: u32);
    fn set_component_color_sys(entity_id: u64, component_id: u64, data_ptr: u32, data_len: u32);
    fn set_component_none_sys(entity_id: u64, component_id: u64);
    fn set_component_sprite_sys(entity_id: u64, handle: u64);
    fn get_component_raw_sys(entity_id: u64, component_id: u64) -> i32;
    fn get_component_obj_sys(entity_id: u64, component_id: u64) -> i32;
    fn get_component_string_sys(entity_id: u64, component_id: u64) -> i32;
    fn get_entities_with_architype_sys(architype_ptr: u32, architype_len: u32);
    fn register_query_sys(ptr: u32, len: u32);
}

// #[macro_export]
// macro_rules! register_query {
//     ($name:ident, $expression:expr) => {
//         #[no_mangle]
//         pub extern "C" fn $name() {}
//     };
// }

// register_query!(test, {});
