pub mod lasers;
use bevy::prelude::App;

pub(crate) fn register(app: &mut App) {
    lasers::register(app);
}
