//! Contains all the server logic related to multiblocks

use bevy::prelude::App;

pub mod reactor;

pub(super) fn register(app: &mut App) {
    reactor::register(app);
}
