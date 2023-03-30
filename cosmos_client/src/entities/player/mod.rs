use bevy::prelude::App;

pub mod render_distance;

pub(super) fn register(app: &mut App) {
    render_distance::register(app);
}
