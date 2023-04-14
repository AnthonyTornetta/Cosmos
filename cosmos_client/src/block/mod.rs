use bevy::prelude::App;

pub mod lighting;

pub(super) fn register(app: &mut App) {
    lighting::register(app);
}
