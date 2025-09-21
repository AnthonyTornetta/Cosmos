//! The shipyard multiblock logic

use bevy::prelude::*;

mod impls;

pub(super) fn register(app: &mut App) {
    impls::register(app);
}
