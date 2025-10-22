use bevy::prelude::*;

use serde::{Deserialize, Serialize};

use crate::netty::sync::{IdentifiableComponent, SyncableComponent, sync_component};

#[derive(Component, Serialize, Deserialize, Debug, Reflect, Clone, Copy, PartialEq)]
pub struct ItemCooldown(pub f32);

impl IdentifiableComponent for ItemCooldown {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:item_cooldown"
    }
}

impl SyncableComponent for ItemCooldown {
    fn validate(&self) -> bool {
        self.0 >= 0.0 && self.0 <= 1.0
    }

    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<ItemCooldown>(app);

    app.register_type::<ItemCooldown>();
}
