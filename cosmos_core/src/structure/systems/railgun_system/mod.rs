//! Represents all the mining lasers on a structure

use bevy::prelude::*;
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::block::block_direction::BlockDirection;
use crate::prelude::BlockCoordinate;

use super::sync::SyncableSystem;
use super::{StructureSystemImpl, StructureSystemsSet};

pub enum InvalidRailgunReason {
    NoPower,
    NoMagnets,
    TouchingAnother,
}

#[derive(Serialize, Deserialize, Debug, Reflect)]
pub struct Railgun {
    pub origin: BlockCoordinate,
    pub direction: BlockDirection,
    pub length: u32,
    pub capacitance: u32,
    pub energy_stored: u32,
    pub valid: bool,
}

#[derive(Serialize, Deserialize, Debug, Component, Reflect, Default)]
pub struct RailgunSystem {
    pub railguns: Vec<Railgun>,
}

impl StructureSystemImpl for RailgunSystem {
    fn unlocalized_name() -> &'static str {
        "cosmos:railgun"
    }
}

impl SyncableSystem for RailgunSystem {}

fn name_railgun_system(mut commands: Commands, q_added: Query<Entity, Added<RailgunSystem>>) {
    for e in q_added.iter() {
        commands.entity(e).insert(Name::new("Railgun System"));
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<RailgunSystem>().add_systems(
        Update,
        name_railgun_system
            .ambiguous_with_all() // doesn't matter if this is 1-frame delayed
            .after(StructureSystemsSet::InitSystems),
    );
}
