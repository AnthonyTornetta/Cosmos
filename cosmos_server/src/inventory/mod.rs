//! Server inventory management

use bevy::prelude::App;

mod netty;

pub(super) fn register(app: &mut App) {
    netty::register(app);
}
