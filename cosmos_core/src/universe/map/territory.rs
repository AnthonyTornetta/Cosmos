//! Tracks the territory contained by a faction

use bevy::{platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    ecs::name,
    faction::FactionId,
    netty::sync::{IdentifiableComponent, SyncableComponent, sync_component},
    physics::location::SystemCoordinate,
};

#[derive(Component, Debug, Reflect, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Tracks the territory claimed by the factions
///
/// Only one faction can own a specific system coordinate
pub struct FactionClaimedTerritory(HashMap<SystemCoordinate, FactionId>);

impl FactionClaimedTerritory {
    /// Iterates over all owned system coordinates
    pub fn iter(&self) -> impl Iterator<Item = (&SystemCoordinate, &FactionId)> {
        self.0.iter()
    }

    /// Assigns the faction to be the owner of this system - removing any previous owner
    pub fn claim(&mut self, system: SystemCoordinate, fac: FactionId) {
        self.0.insert(system, fac);
    }

    /// Checks if this system is claimed by any faction
    pub fn is_claimed(&self, system: SystemCoordinate) -> bool {
        self.0.contains_key(&system)
    }

    /// Gets the claim (if any) for this system
    pub fn get_claim(&self, system: SystemCoordinate) -> Option<FactionId> {
        self.0.get(&system).copied()
    }
}

impl IdentifiableComponent for FactionClaimedTerritory {
    fn get_component_unlocalized_name() -> &'static str {
        "cosoms:faction_claimed_territory"
    }
}

impl SyncableComponent for FactionClaimedTerritory {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<FactionClaimedTerritory>(app);
    app.add_systems(Update, name::<FactionClaimedTerritory>("Galaxy"));
}
