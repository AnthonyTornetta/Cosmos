//! The shipyard multiblock logic

use bevy::prelude::*;
use cosmos_core::prelude::FullStructure;

mod impls;

pub struct ShipyardBeingGenerated {
    structure: FullStructure,
    entity: Entity,
    // Amount of blocks already placed
    already_done: u32,
}

#[derive(Component, Debug)]
pub struct StructureBeingBuilt;

pub(super) fn register(app: &mut App) {
    impls::register(app);
}
