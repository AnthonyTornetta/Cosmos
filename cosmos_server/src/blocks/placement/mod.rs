use bevy::prelude::*;

mod rules;

pub(super) fn register(app: &mut App) {
    rules::register(app);
}
