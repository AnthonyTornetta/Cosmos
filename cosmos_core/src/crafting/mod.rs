use bevy::prelude::App;

pub mod recipes;

pub(super) fn register(app: &mut App) {
    recipes::register(app);
}
