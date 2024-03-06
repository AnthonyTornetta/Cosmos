use bevy::prelude::App;

pub mod terrain_generation;

pub(super) fn register(app: &mut App) {
    terrain_generation::register(app);
}
