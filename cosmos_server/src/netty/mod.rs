//! All server network-related code

use bevy::prelude::App;

pub mod network_helpers;
pub mod player_filtering;
pub mod server_events;
pub mod server_listener;
pub mod sync;

pub(super) fn register(app: &mut App) {
    sync::register(app);
    server_events::register(app);
    server_listener::register(app);
    player_filtering::register(app);
}
