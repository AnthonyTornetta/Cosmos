//! Shared warp logic

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::{IdentifiableComponent, SyncableComponent, sync_component},
    physics::location::Location,
};

#[derive(Component, Debug, Reflect, Serialize, Deserialize, PartialEq, Clone, Copy)]
/// Set by the client to indicate where they want to go
pub struct DesiredLocation(pub Option<Location>);

impl IdentifiableComponent for DesiredLocation {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:desired_location"
    }
}

impl SyncableComponent for DesiredLocation {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ClientAuthoritative(crate::netty::sync::ClientAuthority::Piloting)
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<DesiredLocation>(app);

    app.register_type::<DesiredLocation>();
}
