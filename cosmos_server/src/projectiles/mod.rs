use bevy::prelude::App;

pub mod laser;

fn laser_hit_block() {}

pub(crate) fn register(app: &mut App) {
    laser::register(app);
}
