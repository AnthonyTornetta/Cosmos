//! Client-side ship systems logic

mod camera_system;
mod energy_generation_system;
mod energy_storage_system;
pub mod laser_cannon_system;
pub mod mining_laser_system;
pub mod player_interactions;
mod sync;
pub mod thruster_system;

use bevy::prelude::App;

pub(super) fn register(app: &mut App) {
    player_interactions::register(app);
    thruster_system::register(app);
    camera_system::register(app);
    laser_cannon_system::register(app);
    mining_laser_system::register(app);
    energy_generation_system::register(app);
    energy_storage_system::register(app);
    sync::register(app);
}
