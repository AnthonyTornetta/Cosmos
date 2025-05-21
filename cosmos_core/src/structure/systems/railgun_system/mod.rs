//! Represents all the mining lasers on a structure

use bevy::prelude::*;
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::block::block_direction::BlockDirection;
use crate::netty::sync::IdentifiableComponent;
use crate::netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl};
use crate::prelude::BlockCoordinate;

use super::sync::SyncableSystem;
use super::{StructureSystemImpl, StructureSystemsSet};

#[derive(Serialize, Deserialize, Debug, Reflect, Clone, Copy)]
pub enum InvalidRailgunReason {
    NoMagnets,
    TouchingAnother,
    Obstruction,
    NoCapacitors,
    NoCooling,
}

#[derive(Component, Serialize, Deserialize, Debug, Reflect, Default, Clone)]
pub struct RailgunBlock {
    pub energy_stored: u32,
    pub heat: f32,
}

impl IdentifiableComponent for RailgunBlock {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:railgun"
    }
}

impl RailgunBlock {
    /// Returns [`None`] if ready to fire, or some [`RailgunFailureReason`] if unable to fire.
    pub fn get_unready_reason(&self, railgun_system_entry: &RailgunSystemEntry) -> Option<RailgunFailureReason> {
        if railgun_system_entry.invalid_reason.is_some() {
            return Some(RailgunFailureReason::InvalidStructure);
        }
        if railgun_system_entry.capacitance > self.energy_stored {
            return Some(RailgunFailureReason::LowPower);
        }
        if self.heat.round() as u32 + railgun_system_entry.heat_per_fire > railgun_system_entry.max_heat {
            return Some(RailgunFailureReason::TooHot);
        }

        None
    }
}

#[derive(Serialize, Deserialize, Debug, Reflect, Default, Clone)]
pub struct RailgunSystemEntry {
    pub origin: BlockCoordinate,
    pub direction: BlockDirection,
    pub length: u32,
    pub capacitance: u32,
    /// Watts
    pub charge_rate: f32,
    pub invalid_reason: Option<InvalidRailgunReason>,
    pub max_heat: u32,
    pub cooling_rate: f32,
    pub heat_per_fire: u32,
}

pub enum RailgunFailureReason {
    LowPower,
    TooHot,
    InvalidStructure,
}

impl RailgunSystemEntry {
    pub fn is_valid_structure(&self) -> bool {
        self.invalid_reason.is_none()
    }
}

#[derive(Serialize, Deserialize, Debug, Component, Reflect, Default)]
pub struct RailgunSystem {
    pub railguns: Vec<RailgunSystemEntry>,
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
    app.add_netty_event::<RailgunFiredEvent>().register_type::<RailgunBlock>();

    app.register_type::<RailgunSystem>().add_systems(
        Update,
        name_railgun_system
            .ambiguous_with_all() // doesn't matter if this is 1-frame delayed
            .after(StructureSystemsSet::InitSystems),
    );
}
