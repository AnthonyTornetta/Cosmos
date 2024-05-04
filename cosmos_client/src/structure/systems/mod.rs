//! Client-side ship systems logic

mod camera_system;
mod dock_system;
mod energy_generation_system;
mod energy_storage_system;
pub mod laser_cannon_system;
pub mod mining_laser_system;
pub mod missile_launcher_system;
pub mod player_interactions;
mod shield_system;
mod sync;
pub mod thruster_system;

use bevy::prelude::App;

pub(super) fn register(app: &mut App) {
    dock_system::register(app);
    player_interactions::register(app);
    shield_system::register(app);
    thruster_system::register(app);
    camera_system::register(app);
    laser_cannon_system::register(app);
    mining_laser_system::register(app);
    energy_generation_system::register(app);
    energy_storage_system::register(app);
    missile_launcher_system::register(app);
    sync::register(app);
}
