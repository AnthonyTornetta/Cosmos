use bevy::prelude::App;

pub mod server_plugin;
mod vizualizer;

pub(super) fn register(app: &mut App) {
    vizualizer::register(app);
}
