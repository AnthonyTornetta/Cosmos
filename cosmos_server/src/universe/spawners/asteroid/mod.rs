use bevy::prelude::*;

mod dynamic_spawner;
mod fixed_spawner;

pub(super) fn register(app: &mut App) {
    fixed_spawner::register(app);
    dynamic_spawner::register(app);
}
