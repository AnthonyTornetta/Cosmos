use bevy::prelude::App;

pub mod lighting;

pub(crate) fn register(app: &mut App) {
    lighting::register(app);
}
