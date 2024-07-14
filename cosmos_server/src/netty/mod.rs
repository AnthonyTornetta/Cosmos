//! All server network-related code

use bevy::prelude::App;
use cosmos_core::netty::sync::SyncedComponentId;

use crate::registry::sync_registry;

pub mod network_helpers;
pub mod server_events;
pub mod server_listener;
pub mod sync;

pub(super) fn register(app: &mut App) {
    // TODO: Move this to core project.
    sync_registry::<SyncedComponentId>(app);

    sync::register(app);
    server_events::register(app);
    server_listener::register(app);
}
