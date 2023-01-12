use bevy::prelude::App;

pub mod sync;

pub fn register(app: &mut App) {
    sync::register(app);
}
