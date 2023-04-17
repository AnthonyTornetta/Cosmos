//! Server inventory management

use bevy::prelude::App;

mod sync;

pub(super) fn register(app: &mut App) {
    sync::register(app);
}
