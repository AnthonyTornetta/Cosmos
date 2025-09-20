//! Contains all the server logic related to multiblocks

use bevy::prelude::App;

pub mod checker;
pub mod reactor;
pub mod shipyard;

pub(super) fn register(app: &mut App) {
    reactor::register(app);
    shipyard::register(app);
}
