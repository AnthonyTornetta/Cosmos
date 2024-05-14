use bevy::{app::App, ecs::component::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

use crate::netty::sync::{sync_component, ClientAuthority, SyncableComponent};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Component, PartialEq, Eq, Reflect)]
pub struct HeldItemSlot(u32);

impl HeldItemSlot {
    pub fn new(slot: u32) -> Option<Self> {
        let x = Self(slot);
        if !x.validate() {
            None
        } else {
            Some(x)
        }
    }

    pub fn slot(&self) -> u32 {
        self.0
    }

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

impl SyncableComponent for HeldItemSlot {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::BothAuthoritative(ClientAuthority::Themselves)
    }

    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:held_item_slot"
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
