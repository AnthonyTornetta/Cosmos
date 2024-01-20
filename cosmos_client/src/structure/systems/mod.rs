//! Client-side ship systems logic

pub mod laser_cannon_system;
pub mod mining_laser_system;
mod player_interactions;
pub mod thruster_system;

use bevy::prelude::*;

#[derive(Component)]
#[component(storage = "SparseSet")]
struct ActivatingSelectedSystem;

pub(super) fn register(app: &mut App) {
    player_interactions::register(app);
    thruster_system::register(app);
    laser_cannon_system::register(app);
    mining_laser_system::register(app);
}
