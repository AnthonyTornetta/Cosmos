use bevy::prelude::App;

pub mod star;

pub(super) fn register(app: &mut App) {
    star::register(app);
}
