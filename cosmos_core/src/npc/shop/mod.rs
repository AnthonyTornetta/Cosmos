use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    entities::EntityId,
    faction::FactionId,
    netty::sync::{IdentifiableComponent, SyncableComponent, sync_component},
    physics::location::Location,
};

#[derive(Component, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShopNpc;

impl IdentifiableComponent for ShopNpc {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:shop_npc"
    }
}

impl SyncableComponent for ShopNpc {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Component)]
pub struct Bounties(Vec<Bounty>);

pub struct Bounty {
    kind: BountyKind,
    payout: u32,
    relations_increase: u32,
    difficulty: Option<BountyDifficulty>,
    location: Location,
    description: String,
}

pub enum BountyDifficulty {
    Easy,
    Medium,
    Hard,
    Insane,
}

pub enum BountyKind {
    Pirate {
        n_pirates: u32,
    },
    /// This bounty is placed on another faction
    Faction {
        other_faction: FactionId,
        relations_decrease: u32,
    },
    Player {
        id: EntityId,
    },
}

pub(super) fn register(app: &mut App) {
    sync_component::<ShopNpc>(app);

    // app.add_systems(schedule, systems)
}
