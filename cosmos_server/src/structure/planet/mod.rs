use bevy::prelude::App;

pub mod biosphere;
pub mod generation;
pub mod server_planet_builder;

pub(crate) fn register(app: &mut App) {
    biosphere::register(app);
}
