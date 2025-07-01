use bevy::prelude::*;

mod build_a_ship;
mod collect_stash;

pub(super) fn register(app: &mut App) {
    build_a_ship::register(app);
    collect_stash::register(app);
}
