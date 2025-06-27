//! Thruster block system

use bevy::{
    platform::collections::HashMap,
    prelude::{App, Component, Resource},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{block::Block, registry::identifiable::Identifiable};

use super::{StructureSystemImpl, sync::SyncableSystem};

/// A block that is a thruster will have a thruster property
pub struct ThrusterProperty {
    /// How much thrust this block generates
    pub strength: f32,
    /// How much energy this block consumes
    pub energy_consupmtion: f32,
}

#[derive(Default, Resource)]
/// All blocks that are thruster blocks should be registered here
pub struct ThrusterBlocks {
    blocks: HashMap<u16, ThrusterProperty>,
}

impl ThrusterBlocks {
    /// Inserts a new entry into the registry
    pub fn insert(&mut self, block: &Block, thruster: ThrusterProperty) {
        self.blocks.insert(block.id(), thruster);
    }

    /// Gets an entry from the registry if it exists
    pub fn get(&self, block: &Block) -> Option<&ThrusterProperty> {
        self.blocks.get(&block.id())
    }
}

#[derive(Component, Default, Reflect, Serialize, Deserialize, Debug)]
/// Represents all the thruster blocks on this structure
pub struct ThrusterSystem {
    thrust_total: f32,
    energy_consumption: f32,
}

impl StructureSystemImpl for ThrusterSystem {
    fn unlocalized_name() -> &'static str {
        "cosmos:thruster_system"
    }
}

impl SyncableSystem for ThrusterSystem {}

impl ThrusterSystem {
    /// Called whenever a block is added
    pub fn block_removed(&mut self, old_prop: &ThrusterProperty) {
        self.energy_consumption -= old_prop.energy_consupmtion;
        self.thrust_total -= old_prop.strength;
    }

    /// Called whenever a block is removed
    pub fn block_added(&mut self, prop: &ThrusterProperty) {
        self.energy_consumption += prop.energy_consupmtion;
        self.thrust_total += prop.strength;
    }

    /// Total amount of force exerted on the ship per second while the system is running
    pub fn thrust_total(&self) -> f32 {
        self.thrust_total
    }

    /// Amount of energy used per second to run the thruster system
    pub fn energy_consumption(&self) -> f32 {
        self.energy_consumption
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(ThrusterBlocks::default()).register_type::<ThrusterSystem>();
}
