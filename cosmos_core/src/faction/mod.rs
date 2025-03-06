use std::fs;

use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entities::EntityId,
    netty::{
        cosmos_encoder,
        sync::{
            resources::{sync_resource, SyncableResource},
            sync_component, IdentifiableComponent, SyncableComponent,
        },
        system_sets::NetworkingSystemsSet,
    },
    state::GameState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Default)]
pub enum FactionRelation {
    Ally,
    #[default]
    Neutral,
    Enemy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FactionSettings {
    /// If this is true, this faction will automatically be at war with any neutral faction.
    pub default_enemy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Faction {
    id: FactionId,
    name: String,
    players: Vec<EntityId>,
    relationships: HashMap<FactionId, FactionRelation>,
    settings: FactionSettings,
}

impl Faction {
    pub fn new(
        name: String,
        players: Vec<EntityId>,
        relationships: HashMap<FactionId, FactionRelation>,
        settings: FactionSettings,
    ) -> Self {
        Self {
            id: FactionId::generate_new(),
            name,
            players,
            relationships,
            settings,
        }
    }

    pub fn id(&self) -> &FactionId {
        &self.id
    }

    pub fn relation_with_option(a: Option<&Faction>, b: Option<&Faction>) -> FactionRelation {
        if let Some(a) = a {
            a.relation_with(b)
        } else if let Some(b) = b {
            b.relation_with(a)
        } else {
            FactionRelation::Neutral
        }
    }

    pub fn relation_with(&self, other_faction: Option<&Faction>) -> FactionRelation {
        if let Some(other_faction) = other_faction {
            if self.id == other_faction.id {
                FactionRelation::Ally
            } else if let Some(rel) = self.relationships.get(&other_faction.id) {
                *rel
            } else if let Some(rel) = other_faction.relationships.get(&self.id) {
                *rel
            } else {
                if self.settings.default_enemy || other_faction.settings.default_enemy {
                    FactionRelation::Enemy
                } else {
                    FactionRelation::Neutral
                }
            }
        } else {
            if self.settings.default_enemy {
                FactionRelation::Enemy
            } else {
                FactionRelation::Neutral
            }
        }
    }
}

#[derive(Clone, Copy, Component, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Default)]
/// Links this entity to the faction its apart of.
pub struct FactionId(Uuid);

impl FactionId {
    pub fn generate_new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl IdentifiableComponent for FactionId {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:faction_id"
    }
}

impl SyncableComponent for FactionId {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Resource, Reflect, Clone, Serialize, Deserialize, Debug, Default)]
pub struct Factions(HashMap<FactionId, Faction>);

impl Factions {
    pub fn add_new_faction(&mut self, faction: Faction) {
        self.0.insert(faction.id, faction);
    }

    pub fn from_id(&self, id: &FactionId) -> Option<&Faction> {
        self.0.get(id)
    }

    pub fn from_name(&self, name: &str) -> Option<&Faction> {
        self.0.values().find(|x| x.name == name)
    }
}

impl SyncableResource for Factions {
    fn unlocalized_name() -> &'static str {
        "cosmos:factions"
    }
}

pub(super) fn register(app: &mut App) {
    sync_resource::<Factions>(app);
    sync_component::<FactionId>(app);

    app.register_type::<FactionRelation>()
        .register_type::<Faction>()
        .register_type::<Uuid>()
        .register_type::<FactionId>();
}
