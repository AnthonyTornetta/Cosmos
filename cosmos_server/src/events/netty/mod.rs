// Handles client disconnects/connections

use bevy::prelude::App;

pub mod netty_events;

pub(super) fn register(app: &mut App) {
    netty_events::register(app);
}
