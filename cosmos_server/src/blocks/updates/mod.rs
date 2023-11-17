use bevy::prelude::App;

mod grass_update;

pub(super) fn register(app: &mut App) {
    grass_update::register(app);
}
