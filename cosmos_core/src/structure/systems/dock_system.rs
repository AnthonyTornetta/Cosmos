//! Represents all the energy stored on a structure

use bevy::{
    prelude::{App, Component},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::structure::coordinates::BlockCoordinate;

use super::{sync::SyncableSystem, StructureSystemImpl};

#[derive(Component, Default, Reflect, Serialize, Deserialize, Debug)]
/// Represents the energy storage of a structure
pub struct DockSystem {
    docking_blocks: Vec<BlockCoordinate>,
}

impl SyncableSystem for DockSystem {}

impl StructureSystemImpl for DockSystem {
    fn unlocalized_name() -> &'static str {
        "cosmos:dock_system"
    }
}

impl DockSystem {
    /// Call this whenever a block is added to the system
    pub fn block_added(&mut self, location: BlockCoordinate) {
        self.docking_blocks.push(location)
    }

    /// Call this whenever a block is removed from the system
    pub fn block_removed(&mut self, location: BlockCoordinate) {
        let Some((idx, _)) = self.docking_blocks.iter().enumerate().find(|(_, &x)| x == location) else {
            return;
        };

        self.docking_blocks.remove(idx);
    }

    /// Returns all the camera locations
    pub fn block_locations(&self) -> &[BlockCoordinate] {
        self.docking_blocks.as_slice()
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<DockSystem>();
}
