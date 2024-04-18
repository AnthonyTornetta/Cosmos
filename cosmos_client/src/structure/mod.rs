//! Handles client-related structure things

use bevy::prelude::App;

pub mod asteroid;
mod audio;
pub mod chunk_retreiver;
pub mod client_structure_builder;
mod events;
pub mod planet;
pub mod shared;
mod shields;
pub mod ship;
pub mod station;
pub mod systems;

pub(super) fn register(app: &mut App) {
    systems::register(app);
    chunk_retreiver::register(app);
    ship::register(app);
    planet::register(app);
    asteroid::register(app);
    audio::register(app);
    events::register(app);
    shared::register(app);
    shields::register(app);
    station::register(app);
}
