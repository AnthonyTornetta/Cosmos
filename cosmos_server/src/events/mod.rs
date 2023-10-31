//! This should be removed since a master 'events' module isn't that great

use bevy::prelude::App;

pub mod create_ship_event;
pub mod netty;
pub mod structure;

pub(super) fn register(app: &mut App) {
    create_ship_event::register(app);
    structure::regsiter(app);
    netty::register(app);
}
