use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    ecs::name,
    netty::sync::{IdentifiableComponent, SyncableComponent, sync_component},
    prelude::BlockCoordinate,
    structure::systems::{StructureSystemImpl, sync::SyncableSystem},
};

#[derive(Component, Default, Reflect, Serialize, Deserialize, Debug, Clone)]
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
pub struct WarpBlockProperty {
    pub charge_per_tick: u32,
    pub capacitance: u32,
}

#[derive(Component, Default, Reflect, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WarpDriveInitiating {
    pub charge: f32,
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

impl WarpDriveSystem {
    pub fn can_jump(&self, structure_mass: f32) -> bool {
        Self::compute_jump_charge(structure_mass) <= self.charge
    }

    pub fn compute_jump_charge(structure_mass: f32) -> u32 {
        // ((structure_mass * 10.0).ceil() as u32).max(10_000)
        100
    }

    pub fn empty(&self) -> bool {
        self.max_charge == 0
    }

    pub fn charge(&self) -> u32 {
        self.charge
    }

    pub fn max_charge(&self) -> u32 {
        self.max_charge
    }

    pub fn increase_charge(&mut self, amt: u32) {
        self.charge += amt
    }

    pub fn decrease_charge(&mut self, amt: u32) -> bool {
        if self.charge >= amt {
            self.charge -= amt;
            true
        } else {
            false
        }
    }

    pub fn discharge(&mut self) {
        self.charge = 0;
    }

    pub fn charge_per_tick(&self) -> u32 {
        self.charge_per_tick
    }

    pub fn increase_max_charge(&mut self, amount: u32) {
        self.max_charge += amount;
    }

    pub fn decrease_max_charge(&mut self, amount: u32) {
        self.max_charge -= amount.min(self.max_charge)
    }

    pub fn add_warp_block(&mut self, coordinate: BlockCoordinate, property: WarpBlockProperty) {
        if !self.warp_blocks.contains(&coordinate) {
            self.warp_blocks.push(coordinate);
        }
        self.increase_charge_per_tick(property.charge_per_tick);
        self.increase_max_charge(property.capacitance);
    }

    pub fn remove_warp_block(&mut self, coordinate: BlockCoordinate, property: WarpBlockProperty) {
        if let Some((idx, _)) = self.warp_blocks.iter().enumerate().find(|(_, b)| **b == coordinate) {
            self.warp_blocks.remove(idx);
        }
        self.decrease_charge_per_tick(property.charge_per_tick);
        self.decrease_max_charge(property.capacitance);
    }

    pub fn increase_charge_per_tick(&mut self, amt: u32) {
        self.charge_per_tick += amt
    }

    pub fn decrease_charge_per_tick(&mut self, amt: u32) {
        self.charge_per_tick -= amt.min(self.charge_per_tick)
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<WarpDriveInitiating>(app);

    app.register_type::<WarpDriveSystem>()
        .register_type::<WarpDriveInitiating>()
        .add_systems(Update, name::<WarpDriveSystem>("Warp Drive System"));
}
