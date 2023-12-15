//! This should be removed since a master 'events' module isn't that great

use bevy::prelude::App;

pub mod netty;

pub(super) fn register(app: &mut App) {
    netty::register(app);
}
