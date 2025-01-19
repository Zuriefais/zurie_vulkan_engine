use std::path::Path;

use super::ScriptingState;
use zurie_render::sprite::LoadSpriteInfo;
use zurie_render::sprite::SpriteManager;
use zurie_shared::slotmap::Key;
use zurie_shared::slotmap::KeyData;
use zurie_types::ComponentData;
use zurie_types::SpriteHandle as EngineSpriteHandle;

use crate::functions::zurie::engine::core::EntityId;
use crate::functions::zurie::engine::core::SpriteHandle;
use crate::functions::zurie::engine::sprite;

impl sprite::Host for ScriptingState {
    fn load_sprite_file(&mut self, path: String) -> SpriteHandle {
        KeyData::as_ffi(
            self.sprite_manager
                .write()
                .unwrap()
                .push_to_load_queue(LoadSpriteInfo::Path(Box::from(Path::new(&path))))
                .data(),
        )
    }

    fn load_sprite_bin(&mut self, bin: Vec<u8>) -> SpriteHandle {
        KeyData::as_ffi(
            self.sprite_manager
                .write()
                .unwrap()
                .push_to_load_queue(LoadSpriteInfo::Buffer(bin))
                .data(),
        )
    }

    fn set_sprite(&mut self, entity: EntityId, sprite: SpriteHandle) {
        self.world.write().unwrap().set_component(
            KeyData::from_ffi(entity).into(),
            (self.sprite_component, ComponentData::Sprite(sprite)),
        );
    }

    fn remove_sprite(&mut self, entity: EntityId) {
        self.world
            .write()
            .unwrap()
            .remove_component(KeyData::from_ffi(entity).into(), self.sprite_component);
    }
}
