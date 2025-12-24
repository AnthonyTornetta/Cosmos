//! The warp drive used by ships

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    ecs::name,
    netty::sync::{
        IdentifiableComponent, SyncableComponent,
        events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
        sync_component,
    },
    prelude::BlockCoordinate,
    structure::systems::{StructureSystemImpl, sync::SyncableSystem},
};

#[derive(Component, Default, Reflect, Serialize, Deserialize, Debug, Clone)]
/// The warp drive used by ships
pub struct WarpDriveSystem {
    charge: u32,
    max_charge: u32,
    charge_per_tick: u32,
    warp_blocks: Vec<BlockCoordinate>,
}

impl StructureSystemImpl for WarpDriveSystem {
    fn unlocalized_name() -> &'static str {
        "cosmos:warp_drive_system"
    }
}

impl SyncableSystem for WarpDriveSystem {}

#[derive(Debug, Clone, Copy)]
/// A block that can be used to power a ship's warp drive
pub struct WarpBlockProperty {
    /// The amount of energy this pulls to charge itself per tick
    pub charge_per_tick: u32,
    /// The amount of charge this can hold
    pub capacitance: u32,
}

#[derive(Component, Default, Reflect, Serialize, Deserialize, Debug, Clone, PartialEq)]
/// Indicates the warp drive system on this ship is undergoing the warp sequence
pub struct WarpDriveInitiating {
    /// How far along the warp sequence we are
    pub charge: f32,
    /// The maximum charge before warp is complete
    pub max_charge: f32,
}

impl IdentifiableComponent for WarpDriveInitiating {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:warp_drive_initiating"
    }
}

impl SyncableComponent for WarpDriveInitiating {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

/// A summary of the warp drive's state
pub enum WarpDriveSystemState {
    /// The warp drive is on a structure that is too big for the number of warp drives placed
    StructureTooBig,
    /// The warp drive is fully charged and ready to jump
    ReadyToWarp,
    /// The warp drive is charging, and is on a structure that is small enough to jump
    Charging,
}

impl WarpDriveSystem {
    /// Returns an easy to use state of this warp system based on its structure's mass
    pub fn compute_state(&self, structure_mass: f32) -> WarpDriveSystemState {
        if self.can_jump(structure_mass) {
            WarpDriveSystemState::ReadyToWarp
        } else if Self::compute_jump_charge(structure_mass) > self.max_charge {
            WarpDriveSystemState::StructureTooBig
        } else {
            WarpDriveSystemState::Charging
        }
    }

    /// Checks if there is enough charge to jump given this structure's mass
    pub fn can_jump(&self, structure_mass: f32) -> bool {
        Self::compute_jump_charge(structure_mass) <= self.charge
    }

    /// Returns the total amount of charge required for a structure of this mass to jump
    pub fn compute_jump_charge(structure_mass: f32) -> u32 {
        ((structure_mass * 10.0).ceil() as u32).max(10_000)
    }

    /// Checks if this warp drive has no charge
    pub fn empty(&self) -> bool {
        self.max_charge == 0
    }

    /// Computes the total charge this warp drive has
    pub fn charge(&self) -> u32 {
        self.charge
    }

    /// Returns the maximum amount of charge this warp drive can hold
    ///
    /// This is NOT the required warp amount - see [`Self::compute_jump_charge`].
    pub fn max_charge(&self) -> u32 {
        self.max_charge
    }

    /// Increases the amount of charge this warp drive has. Capped at [`Self::max_charge`]
    pub fn increase_charge(&mut self, amt: u32) {
        self.charge += amt;
        if self.charge > self.max_charge {
            self.charge = self.max_charge;
        }
    }

    /// Decreases the charge by this quantity
    ///
    /// If this amount is too large, no charge is removed and false is returned.
    pub fn decrease_charge(&mut self, amt: u32) -> bool {
        if self.charge >= amt {
            self.charge -= amt;
            true
        } else {
            false
        }
    }

    /// Sets this warp drive's charge to 0
    pub fn discharge(&mut self) {
        self.charge = 0;
    }

    /// Returns the total charge per tick this will consume to charge itself
    pub fn charge_per_tick(&self) -> u32 {
        self.charge_per_tick
    }

    /// Increases the total capacitance of this system
    pub fn increase_max_charge(&mut self, amount: u32) {
        self.max_charge += amount;
    }

    /// Decreases the total capacitance of this system - will not underflow
    pub fn decrease_max_charge(&mut self, amount: u32) {
        self.max_charge -= amount.min(self.max_charge)
    }

    /// Adds a warp block to this system and recomputes the needed properties
    pub fn add_warp_block(&mut self, coordinate: BlockCoordinate, property: WarpBlockProperty) {
        if !self.warp_blocks.contains(&coordinate) {
            self.warp_blocks.push(coordinate);
        }
        self.increase_charge_per_tick(property.charge_per_tick);
        self.increase_max_charge(property.capacitance);
    }

    /// Removes a warp block from this system and recomputes the needed properties
    ///
    /// Even if this block was not found on the system, the properties will still be removed
    pub fn remove_warp_block(&mut self, coordinate: BlockCoordinate, property: WarpBlockProperty) {
        if let Some((idx, _)) = self.warp_blocks.iter().enumerate().find(|(_, b)| **b == coordinate) {
            self.warp_blocks.remove(idx);
        }
        self.decrease_charge_per_tick(property.charge_per_tick);
        self.decrease_max_charge(property.capacitance);
    }

    /// Increases the amount of charge this sytem gains per tick
    pub fn increase_charge_per_tick(&mut self, amt: u32) {
        self.charge_per_tick += amt
    }

    /// Decreases the amount of charge this sytem gains per tick - will not underflow
    pub fn decrease_charge_per_tick(&mut self, amt: u32) {
        self.charge_per_tick -= amt.min(self.charge_per_tick)
    }
}

#[derive(Message, Clone, Copy, Serialize, Deserialize, Debug)]
/// Sent when a warp drive has its warp sequence cancelled
///
/// Server -> Client
pub struct WarpCancelledMessage {
    /// The structure that had its warp sequence cancelled
    pub structure_entity: Entity,
}

impl IdentifiableMessage for WarpCancelledMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:warp_cancelled"
    }
}

impl NettyMessage for WarpCancelledMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping
            .client_from_server(&self.structure_entity)
            .map(|structure_entity| Self { structure_entity })
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<WarpDriveInitiating>(app);

    app.register_type::<WarpDriveSystem>()
        .register_type::<WarpDriveInitiating>()
        .add_systems(Update, name::<WarpDriveSystem>("Warp Drive System"))
        .add_netty_message::<WarpCancelledMessage>();
}
