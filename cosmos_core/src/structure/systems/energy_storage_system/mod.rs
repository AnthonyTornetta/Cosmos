//! Represents all the energy stored on a structure

use bevy::{
    prelude::{App, Component, Resource},
    reflect::Reflect,
    utils::HashMap,
};
use serde::{Deserialize, Serialize};

use crate::{block::Block, registry::identifiable::Identifiable};

use super::{sync::SyncableSystem, StructureSystemImpl};

#[derive(Default, Reflect, Clone, Copy)]
/// Every block that can store energy should have this property
pub struct EnergyStorageProperty {
    /// How much energy this block can store
    pub capacity: f32,
}

#[derive(Default, Resource)]
/// All the energy storage blocks - register them here.
pub struct EnergyStorageBlocks {
    blocks: HashMap<u16, EnergyStorageProperty>,
}

impl EnergyStorageBlocks {
    /// Inserts a block with a property
    pub fn insert(&mut self, block: &Block, storage_property: EnergyStorageProperty) {
        self.blocks.insert(block.id(), storage_property);
    }

    /// Gets a property from that block if it has one
    pub fn get(&self, block: &Block) -> Option<&EnergyStorageProperty> {
        self.blocks.get(&block.id())
    }
}

#[derive(Component, Default, Reflect, Serialize, Deserialize, Debug)]
/// Represents the energy storage of a structure
pub struct EnergyStorageSystem {
    energy: f32,
    capacity: f32,
}

impl SyncableSystem for EnergyStorageSystem {}

impl StructureSystemImpl for EnergyStorageSystem {
    fn unlocalized_name() -> &'static str {
        "cosmos:energy_storage_system"
    }
}

impl EnergyStorageSystem {
    /// Call this whenever a block is added to the system
    pub fn block_added(&mut self, prop: &EnergyStorageProperty) {
        self.capacity += prop.capacity;
    }

    /// Call this whenever a block is removed from the system
    pub fn block_removed(&mut self, prop: &EnergyStorageProperty) {
        self.capacity -= prop.capacity;
    }

    /// Increases the energy stored in this system
    pub fn increase_energy(&mut self, delta: f32) {
        self.energy = self.capacity.min(self.energy + delta);
    }

    /// Decreases the energy stored in this system - does not go below 0.
    ///
    /// You can use `get_energy` to see if there is enough to use.
    ///
    /// Returns 0.0 if there is enough power to perform this operation, however much power was not able to be taken if not.
    pub fn decrease_energy(&mut self, delta: f32) -> f32 {
        let amount_left = self.energy - delta;
        self.energy = amount_left.max(0.0);

        if amount_left < 0.0 {
            -amount_left
        } else {
            0.0
        }
    }

    /// Gets the current stored energy of the system
    pub fn get_energy(&self) -> f32 {
        self.energy
    }

    /// Gets the totaly capacity of this system
    pub fn get_capacity(&self) -> f32 {
        self.capacity
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(EnergyStorageBlocks::default())
        .register_type::<EnergyStorageSystem>()
        // This is allowed to be ambiguous, because it will be being replaced in the future, once electric wires are done.
        .allow_ambiguous_component::<EnergyStorageSystem>();
}
