//! All server network-related code

use bevy::prelude::App;

pub mod network_helpers;
pub mod server_listener;
pub mod sync;

pub(super) fn register(app: &mut App) {
    sync::register(app);
    server_listener::register(app);
}
