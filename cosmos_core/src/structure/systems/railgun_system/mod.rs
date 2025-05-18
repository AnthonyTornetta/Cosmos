//! Represents all the mining lasers on a structure

use bevy::prelude::*;
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::block::block_direction::BlockDirection;
use crate::netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl};
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RailgunFiredInfo {
    pub origin: BlockCoordinate,
    pub length: f32,
    pub direction: Vec3,
}

#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct RailgunFiredEvent {
    pub structure: Entity,
    pub railguns: Vec<RailgunFiredInfo>,
}

impl IdentifiableEvent for RailgunFiredEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:railgun_fired"
    }
}

impl NettyEvent for RailgunFiredEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.client_from_server(&self.structure).map(|s| Self {
            structure: s,
            railguns: self.railguns,
        })
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<RailgunFiredEvent>();

    app.register_type::<RailgunSystem>().add_systems(
        Update,
        name_railgun_system
            .ambiguous_with_all() // doesn't matter if this is 1-frame delayed
            .after(StructureSystemsSet::InitSystems),
    );
}
