use bevy::prelude::App;

mod recipes;

pub(super) fn register(app: &mut App) {
    recipes::register(app);
}
