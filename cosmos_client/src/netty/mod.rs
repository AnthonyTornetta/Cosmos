//! Responsible for all the network information the client has

use bevy::prelude::App;

pub mod connect;
pub mod flags;
mod gameplay;
pub mod lobby;
pub mod mapping;

pub(crate) fn register(app: &mut App) {
    gameplay::register(app);
}
