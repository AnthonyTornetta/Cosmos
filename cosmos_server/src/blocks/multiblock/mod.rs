//! Contains all the server logic related to multiblocks

use bevy::prelude::App;

pub mod reactor;
pub mod reactor_persistence;

pub(super) fn register(app: &mut App) {
    reactor::register(app);
    reactor_persistence::register(app);
}
