use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::{IdentifiableComponent, SyncableComponent, sync_component};

pub mod shop;

#[derive(Component, Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
pub struct Npc {
    pub first_name: String,
    pub last_name: String,
}

impl IdentifiableComponent for Npc {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:npc"
    }
}

impl SyncableComponent for Npc {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<Npc>(app);

    shop::register(app);
}
