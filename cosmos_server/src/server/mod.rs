//! Geneneral server-wide operations (such as stopping the server)

use bevy::prelude::*;

pub mod stop;

pub(super) fn register(app: &mut App) {
    stop::register(app);
}
