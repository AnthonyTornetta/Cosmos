use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};

use crate::{
    entities::EntityId,
    netty::sync::resources::{sync_resource, SyncableResource},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Default)]
pub enum FactionRelation {
    Ally,
    #[default]
    Neutral,
    Enemy,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Faction {
    id: u64,
    name: String,
    players: Vec<EntityId>,
}

#[derive(Clone, Copy, Component, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Default)]
/// Links this entity to the faction its apart of.
pub struct FactionId(u64);

#[derive(Resource, Reflect, Clone, Serialize, Deserialize, Debug, Default)]
pub struct Factions(HashMap<u64, Faction>);

impl SyncableResource for Factions {
    fn unlocalized_name() -> &'static str {
        "cosmos:factions"
    }
}

pub(super) fn register(app: &mut App) {
    sync_resource::<Factions>(app);

    app.register_type::<FactionRelation>()
        .register_type::<Faction>()
        .register_type::<FactionId>();
}
