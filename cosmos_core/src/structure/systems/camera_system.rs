//! Represents all the energy stored on a structure

use bevy::{platform::collections::HashSet, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{block::Block, registry::identifiable::Identifiable, structure::coordinates::BlockCoordinate};

use super::{StructureSystemImpl, sync::SyncableSystem};

#[derive(Default, Resource)]
/// All the energy storage blocks - register them here.
pub struct CameraBlocks {
    blocks: HashSet<u16>,
}

impl CameraBlocks {
    /// Inserts a block with a property
    pub fn insert(&mut self, block: &Block) {
        self.blocks.insert(block.id());
    }

    /// Gets a property from that block if it has one
    pub fn is_camera(&self, block: &Block) -> bool {
        self.blocks.contains(&block.id())
    }
}

#[derive(Component, Default, Reflect, Serialize, Deserialize, Debug)]
/// Represents the energy storage of a structure
pub struct CameraSystem {
    cameras: Vec<BlockCoordinate>,
}

impl SyncableSystem for CameraSystem {}

impl StructureSystemImpl for CameraSystem {
    fn unlocalized_name() -> &'static str {
        "cosmos:camera_system"
    }
}

impl CameraSystem {
    /// Call this whenever a block is added to the system
    pub fn block_added(&mut self, location: BlockCoordinate) {
        self.cameras.push(location)
    }

    /// Call this whenever a block is removed from the system
    pub fn block_removed(&mut self, location: BlockCoordinate) {
        let Some((idx, _)) = self.cameras.iter().enumerate().find(|(_, x)| **x == location) else {
            return;
        };

        self.cameras.remove(idx);
    }

    /// Returns all the camera locations
    pub fn camera_locations(&self) -> &[BlockCoordinate] {
        self.cameras.as_slice()
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(CameraBlocks::default()).register_type::<CameraSystem>();
}
