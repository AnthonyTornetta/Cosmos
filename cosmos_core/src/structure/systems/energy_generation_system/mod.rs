//! Represents all the energy generation in a structure

use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};

use crate::{block::Block, registry::identifiable::Identifiable};

use super::{sync::SyncableSystem, StructureSystemImpl};

#[derive(Component, Default, Reflect, Serialize, Deserialize)]
/// A quick and dirty system that will generate X amount of energy per second.
///
/// This will eventually be removed
pub struct EnergyGenerationSystem {
    generation_rate: f32,
}

impl StructureSystemImpl for EnergyGenerationSystem {
    fn unlocalized_name() -> &'static str {
        "cosmos:energy_generation_system"
    }
}

impl SyncableSystem for EnergyGenerationSystem {}

#[derive(Default, Reflect, Clone, Copy)]
/// Any block that can generate energy will have this property.
pub struct EnergyGenerationProperty {
    /// How much energy is generated
    pub generation_rate: f32,
}

#[derive(Default, Resource)]
/// All the energy generation blocks - register them here.
pub struct EnergyGenerationBlocks {
    blocks: HashMap<u16, EnergyGenerationProperty>,
}

impl EnergyGenerationBlocks {
    /// Inserts a block with a property
    pub fn insert(&mut self, block: &Block, generation_property: EnergyGenerationProperty) {
        self.blocks.insert(block.id(), generation_property);
    }

    /// Gets a property from that block if it has one
    pub fn get(&self, block: &Block) -> Option<&EnergyGenerationProperty> {
        self.blocks.get(&block.id())
    }
}

impl EnergyGenerationSystem {
    /// Call this whenever a block is added to the system
    pub fn block_added(&mut self, prop: &EnergyGenerationProperty) {
        self.generation_rate += prop.generation_rate;
    }

    /// Call this whenever a block is removed from the system
    pub fn block_removed(&mut self, prop: &EnergyGenerationProperty) {
        self.generation_rate -= prop.generation_rate;
    }

    /// How much energy is generated per second
    pub fn energy_generation_rate(&self) -> f32 {
        self.generation_rate
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(EnergyGenerationBlocks::default())
        .register_type::<EnergyGenerationSystem>();
}
