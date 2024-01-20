//! Represents all the energy stored on a structure

use bevy::{
    prelude::{App, Component, Resource},
    reflect::Reflect,
    utils::HashMap,
};

use crate::{block::Block, registry::identifiable::Identifiable};

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

#[derive(Component, Default, Reflect)]
/// Represents the energy storage of a structure
pub struct EnergyStorageSystem {
    energy: f32,
    capacity: f32,
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
    /// Returns true if there is enough power to perform this operation, false if not.
    pub fn decrease_energy(&mut self, delta: f32) -> bool {
        self.energy = (self.energy - delta).max(0.0);
        self.energy > 0.0
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
        .register_type::<EnergyStorageSystem>();
}
