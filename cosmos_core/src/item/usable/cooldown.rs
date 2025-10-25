//! Item cooldown utilities

use bevy::prelude::*;

use serde::{Deserialize, Serialize};

use crate::netty::sync::{IdentifiableComponent, SyncableComponent, sync_component};

#[derive(Component, Serialize, Deserialize, Debug, Reflect, Clone, Copy, PartialEq, Default)]
/// Represents the cooldown of an item's usage
///
/// This will NOT impact anything without you explicitly using it in your systems. Items with this
/// stored as their data will have a cooldown rendered on the client
pub struct ItemCooldown(f32);

impl ItemCooldown {
    /// A number between 0.0 and 1.0 (0% meaning not on cooldown)
    pub fn new(cooldown: f32) -> Self {
        Self(cooldown.clamp(0.0, 1.0))
    }

    /// A number between 0.0 and 1.0 (0% meaning not on cooldown)
    pub fn set(&mut self, cooldown: f32) {
        self.0 = cooldown.clamp(0.0, 1.0);
    }

    /// A number between 0.0 and 1.0 (0% meaning not on cooldown)
    pub fn get(&self) -> f32 {
        self.0
    }
}

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
