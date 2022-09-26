use bevy::prelude::App;

pub mod interactable;

pub fn register(app: &mut App) {
    interactable::register(app);
}
