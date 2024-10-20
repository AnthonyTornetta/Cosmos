use bevy::prelude::{App, Component};
use serde::{Deserialize, Serialize};

use crate::netty::sync::{sync_component, IdentifiableComponent, SyncableComponent};

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Creative;

impl IdentifiableComponent for Creative {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:creative"
    }
}

impl SyncableComponent for Creative {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<Creative>(app);
}
