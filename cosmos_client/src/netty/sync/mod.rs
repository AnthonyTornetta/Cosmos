use bevy::prelude::*;

mod component;

pub(super) fn register(app: &mut App) {
    component::register(app);
}
