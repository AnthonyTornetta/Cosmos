//! Represents all the energy stored on a structure

use bevy::{platform::collections::HashSet, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    block::Block,
    ecs::name,
    registry::identifiable::Identifiable,
    structure::{
        coordinates::BlockCoordinate,
        systems::{SystemActive, SystemEnabled},
    },
};

use super::{StructureSystemImpl, sync::SyncableSystem};

#[derive(Component, Reflect, Clone, Copy)]
pub struct TurretTarget(Entity);

impl TurretTarget {
    pub fn new(target: Entity) -> Self {
        Self(target)
    }

    pub fn get(&self) -> Entity {
        self.0
    }
}

#[derive(Default, Resource)]
/// All the energy storage blocks - register them here.
pub struct TurretBlocks {
    blocks: HashSet<u16>,
}

impl TurretBlocks {
    /// Inserts a block with a property
    pub fn insert(&mut self, block: &Block) {
        self.blocks.insert(block.id());
    }

    /// Gets a property from that block if it has one
    pub fn is_turret(&self, block: &Block) -> bool {
        self.blocks.contains(&block.id())
    }
}

#[derive(Component, Default, Reflect, Serialize, Deserialize, Debug)]
/// Represents the energy storage of a structure
pub struct TurretSystem {
    turrets: Vec<BlockCoordinate>,
}

impl SyncableSystem for TurretSystem {}

impl StructureSystemImpl for TurretSystem {
    fn unlocalized_name() -> &'static str {
        "cosmos:turret_system"
    }
}

impl TurretSystem {
    /// Call this whenever a block is added to the system
    pub fn block_added(&mut self, location: BlockCoordinate) {
        self.turrets.push(location)
    }

    /// Call this whenever a block is removed from the system
    pub fn block_removed(&mut self, location: BlockCoordinate) {
        let Some((idx, _)) = self.turrets.iter().enumerate().find(|(_, x)| **x == location) else {
            return;
        };

        self.turrets.remove(idx);
    }

    /// Returns all the turret locations
    pub fn turret_locations(&self) -> &[BlockCoordinate] {
        self.turrets.as_slice()
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(TurretBlocks::default())
        .register_type::<TurretSystem>()
        .add_systems(Update, name::<TurretSystem>("Turret System"));
}
