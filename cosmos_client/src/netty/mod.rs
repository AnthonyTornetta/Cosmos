//! Responsible for all the network information the client has

use bevy::prelude::App;
use cosmos_core::netty::sync::SyncedComponentId;

use crate::registry::sync_registry;

pub mod connect;
pub mod gameplay;
pub mod lobby;

pub(super) fn register(app: &mut App) {
    // TODO: Move this to core project.
    sync_registry::<SyncedComponentId>(app);

    gameplay::register(app);
}
