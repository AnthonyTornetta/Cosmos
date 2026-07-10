use bevy::prelude::*;

mod build_mode;
mod rules;

pub(super) fn register(app: &mut App) {
    rules::register(app);
    build_mode::register(app);
}
