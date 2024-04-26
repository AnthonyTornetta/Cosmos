//! Contains projectile systems needed on the server

use bevy::prelude::App;
use cosmos_core::{block::BlockRotation, structure::structure_block::StructureBlock};

mod camera_system;
mod energy_generation_system;
mod energy_storage_system;
pub mod laser_cannon_system;
mod line_system;
mod mining_laser_system;
pub mod missile_launcher_system;
mod shield_system;
pub(crate) mod sync;
mod thruster_system;

/// A system that is created by the addition and removal of blocks
pub trait BlockStructureSystem<T> {
    /// Called whenever a block is added that is relevant to this system
    fn add_block(&mut self, sb: &StructureBlock, block_rotation: BlockRotation, property: &T);
    /// Called whenever a block is removed that is relevant to this system
    fn remove_block(&mut self, sb: &StructureBlock);
}

pub(super) fn register(app: &mut App) {
    sync::register(app);
    camera_system::register(app);
    line_system::register(app);
    shield_system::register(app);
    laser_cannon_system::register(app);
    thruster_system::register(app);
    energy_generation_system::register(app);
    mining_laser_system::register(app);
    energy_storage_system::register(app);
    missile_launcher_system::register(app);
}
