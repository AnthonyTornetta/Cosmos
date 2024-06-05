//! Represents the item slot held by the player.

use bevy::{app::App, ecs::component::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

use crate::netty::sync::{sync_component, ClientAuthority, IdentifiableComponent, SyncableComponent};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Component, PartialEq, Eq, Reflect)]
/// Represents the item slot that this player currently is holding
///
/// This is guarenteed to be a valid hotbar slot (0-9)
pub struct HeldItemSlot(u32);

impl HeldItemSlot {
    /// Returns an instance of self if this is a valid hotbar slot (0-9).
    pub fn new(slot: u32) -> Option<Self> {
        let x = Self(slot);
        if !x.validate() {
            None
        } else {
            Some(x)
        }
    }

    /// Returns the slot this is referencing
    pub fn slot(&self) -> u32 {
        self.0
    }

    /// Updates this slot to be the new slot if it is a valid slot (0-9).
    ///
    /// Returns false if that was an invalid slot.
    pub fn set_slot(&mut self, slot: u32) -> bool {
        let old = self.0;
        self.0 = slot;
        if !self.validate() {
            self.0 = old;
            return false;
        }
        true
    }
}

impl IdentifiableComponent for HeldItemSlot {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:held_item_slot"
    }
}

impl SyncableComponent for HeldItemSlot {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::BothAuthoritative(ClientAuthority::Themselves)
    }

    fn validate(&self) -> bool {
        const N_SLOTS_IN_HOTBAR: u32 = 9;
        self.0 < N_SLOTS_IN_HOTBAR
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<HeldItemSlot>(app);

    app.register_type::<HeldItemSlot>();
}
