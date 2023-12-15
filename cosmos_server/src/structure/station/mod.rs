//! Contains server-related station logic

use bevy::prelude::App;

mod sync;

pub(super) fn register(app: &mut App) {
    sync::register(app);
}
