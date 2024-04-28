//! Represents the shield functionality

use bevy::{
    app::App,
    ecs::{component::Component, system::Resource},
    reflect::Reflect,
    utils::HashMap,
};
use serde::{Deserialize, Serialize};

use crate::structure::coordinates::BlockCoordinate;

use super::{sync::SyncableSystem, StructureSystemImpl};

#[derive(Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ShieldProperty {
    pub shield_strength: f32,
    pub shield_range_increase: f32,
}

#[derive(Resource, Default)]
pub struct ShieldBlocks(pub HashMap<u16, ShieldProperty>);

#[derive(Reflect, Default, Component, Clone, Serialize, Deserialize, Debug)]
pub struct ShieldSystem {
    pub shields: Vec<(BlockCoordinate, ShieldProperty)>,
}

impl ShieldSystem {
    pub fn block_removed(&mut self, property: ShieldProperty, coords: BlockCoordinate) {}
    pub fn block_added(&mut self, property: ShieldProperty, coords: BlockCoordinate) {}
}

impl StructureSystemImpl for ShieldSystem {
    fn unlocalized_name() -> &'static str {
        "cosmos:shield"
    }
}

impl SyncableSystem for ShieldSystem {}

pub(super) fn register(app: &mut App) {
    app.register_type::<ShieldSystem>();
}
