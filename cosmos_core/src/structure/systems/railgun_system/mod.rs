//! Represents all the mining lasers on a structure

use bevy::prelude::*;
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::block::block_direction::BlockDirection;
use crate::netty::sync::IdentifiableComponent;
use crate::netty::sync::events::netty_event::{IdentifiableEvent, NettyMessage, SyncedEventImpl};
use crate::prelude::BlockCoordinate;

use super::sync::SyncableSystem;
use super::{StructureSystemImpl, StructureSystemsSet};

#[derive(Serialize, Deserialize, Debug, Reflect, Clone, Copy)]
/// A railgun assembly can fail for all of these reasons
pub enum InvalidRailgunReason {
    /// The railgun either doesn't have enough magnets, or doesn't have enough
    NoMagnets,
    /// This railgun is touching another railgun's blocks
    TouchingAnother,
    /// The cannon of this railgun has a block inside
    Obstruction,
    /// This railgun has no capacitors to charge it
    NoCapacitors,
    /// This railgun has no cooling units to cool it down
    NoCooling,
}

#[derive(Component, Serialize, Deserialize, Debug, Reflect, Default, Clone)]
/// The block data attached to a `cosmos:railgun_launcher` that stores information about the
/// railgun.
pub struct RailgunBlock {
    /// The energy this block contains
    pub energy_stored: u32,
    /// The heat this block contains
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
/// Information about a railgun block and information about it
pub struct RailgunSystemEntry {
    /// The `cosmos:railgun_launcher` block
    pub origin: BlockCoordinate,
    /// The direction this railgun should fire
    pub direction: BlockDirection,
    /// How long this railgun is
    pub length: u32,
    /// How much energy this railgun needs to store before firing
    pub capacitance: u32,
    /// The Watts this railgun can charge at
    pub charge_rate: f32,
    /// Why this railgun is an invalid structure (if it is invalid)
    pub invalid_reason: Option<InvalidRailgunReason>,
    /// The maximum heat this railgun can store
    pub max_heat: u32,
    /// The rate this railgun cools itself down per second
    pub cooling_rate: f32,
    /// The heat this railgun gains after firing
    pub heat_per_fire: u32,
}

/// When a railgun is attempted to be fired, it can fail for these reasons
#[derive(Serialize, Deserialize, Debug, Reflect, Clone, Copy)]
pub enum RailgunFailureReason {
    /// Not enough power to fire this railgun
    LowPower,
    /// This railgun is hot to fire
    TooHot,
    /// This railgun is not valid structurally
    InvalidStructure,
}

impl RailgunSystemEntry {
    /// Returns true if this railgun is a valid railgun structure
    pub fn is_valid_structure(&self) -> bool {
        self.invalid_reason.is_none()
    }
}

#[derive(Serialize, Deserialize, Debug, Component, Reflect, Default)]
/// All the railguns on a structure
pub struct RailgunSystem {
    /// All the railguns on a structure
    ///
    /// TODO: Make this private
    pub railguns: Vec<RailgunSystemEntry>,
}

impl StructureSystemImpl for RailgunSystem {
    fn unlocalized_name() -> &'static str {
        "cosmos:railgun"
    }
}

impl SyncableSystem for RailgunSystem {}

impl RailgunSystem {
    /// Creates a new railgun system based on these railgun entries
    pub fn new(railguns: Vec<RailgunSystemEntry>) -> Self {
        Self { railguns }
    }
}

fn name_railgun_system(mut commands: Commands, q_added: Query<Entity, Added<RailgunSystem>>) {
    for e in q_added.iter() {
        commands.entity(e).insert(Name::new("Railgun System"));
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Information about a railgun that was fired
pub struct RailgunFiredInfo {
    /// The `cosmos:railgun_launcher` block of this railgun
    pub origin: BlockCoordinate,
    /// The length of this railgun shot (how far it travels)
    pub length: f32,
    /// The direction of the shot (not relative to anything)
    pub direction: Vec3,
}

#[derive(Event, Debug, Serialize, Deserialize, Clone)]
/// A railgun was fired
pub struct RailgunFiredEvent {
    /// The structure that fired it
    pub structure: Entity,
    /// All the railguns on this structure that fired
    pub railguns: Vec<RailgunFiredInfo>,
}

impl IdentifiableEvent for RailgunFiredEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:railgun_fired"
    }
}

impl NettyMessage for RailgunFiredEvent {
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
        FixedUpdate,
        name_railgun_system
            .ambiguous_with_all() // doesn't matter if this is 1-frame delayed
            .after(StructureSystemsSet::InitSystems),
    );
}
