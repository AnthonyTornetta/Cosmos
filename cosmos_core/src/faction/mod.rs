//! Factions

use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entities::EntityId,
    netty::sync::{
        resources::{sync_resource, SyncableResource},
        sync_component, IdentifiableComponent, SyncableComponent,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Default)]
/// One faction's relationship with another
pub enum FactionRelation {
    /// These two factions are allies
    Ally,
    #[default]
    /// These two factions are neutral with each other
    Neutral,
    /// These two factions are enemies with each other
    Enemy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
/// Extra configuration for a faction
pub struct FactionSettings {
    /// If this is true, this faction will automatically be at war with any neutral faction.
    pub default_enemy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
/// A collection of players/NPCs under a common team
///
/// Not every player will have a faction.
pub struct Faction {
    id: FactionId,
    name: String,
    players: Vec<EntityId>,
    relationships: HashMap<FactionId, FactionRelation>,
    at_war_with: Vec<EntityId>,
    settings: FactionSettings,
}

impl Faction {
    /// Creates a new faction
    ///
    /// * `relationships` Non-specified factions will be assumed netural, unless the
    ///   [`FactionSettings`] specifies otherwise.
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
            at_war_with: vec![],
            settings,
        }
    }

    /// Returns the [`FactionId`] of this faction
    #[inline(always)]
    pub fn id(&self) -> &FactionId {
        &self.id
    }

    /// Computes the [`FactionRelation`] between two optional factions.
    ///
    /// If both are [`None`], [`FactionRelation::Neutral`] is returned. A convenient wrapper for
    /// [`Self::relation_with`] with that extra check.
    pub fn relation_with_option(a: Option<&Faction>, b: Option<&Faction>) -> FactionRelation {
        if let Some(a) = a {
            a.relation_with(b)
        } else if let Some(b) = b {
            b.relation_with(a)
        } else {
            FactionRelation::Neutral
        }
    }

    /// Computes the relation between this faction and another optional faction. If the other
    /// faction is [`None`], the relationship may be Neutral or Enemy, depending on this
    /// [`Faction`]'s [`FactionSettings`].
    pub fn relation_with_entity(&self, other_entity: &EntityId, other_faction: Option<&Faction>) -> FactionRelation {
        if self.at_war_with.contains(other_entity) {
            FactionRelation::Enemy
        } else {
            self.relation_with(other_faction)
        }
    }

    /// Computes the relation between this faction and another optional faction. If the other
    /// faction is [`None`], the relationship may be Neutral or Enemy, depending on this
    /// [`Faction`]'s [`FactionSettings`].
    pub fn relation_with(&self, other_faction: Option<&Faction>) -> FactionRelation {
        if let Some(other_faction) = other_faction {
            if self.id == other_faction.id {
                FactionRelation::Ally
            } else if let Some(rel) = self.relationships.get(&other_faction.id) {
                *rel
            } else if let Some(rel) = other_faction.relationships.get(&self.id) {
                *rel
            } else if self.settings.default_enemy || other_faction.settings.default_enemy {
                FactionRelation::Enemy
            } else {
                FactionRelation::Neutral
            }
        } else if self.settings.default_enemy {
            FactionRelation::Enemy
        } else {
            FactionRelation::Neutral
        }
    }
}

#[derive(Clone, Copy, Component, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Default)]
/// Links this entity to the faction its apart of.
///
/// ### ⚠️ Important Note:
/// This can point to a no-longer-valid faction! This can occur if a faction is deleted, but this
/// entity wasn't loaded when the faction was deleted. Never assume this is valid without checking
/// the [`Factions`] resource first!
pub struct FactionId(Uuid);

impl FactionId {
    /// Generates a new, unique faction id
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
/// A resource containing every [`Faction`] in the game
pub struct Factions(HashMap<FactionId, Faction>);

impl Factions {
    /// Adds a new faction
    pub fn add_new_faction(&mut self, faction: Faction) {
        self.0.insert(faction.id, faction);
    }

    /// Gets a faction from this id
    pub fn from_id(&self, id: &FactionId) -> Option<&Faction> {
        self.0.get(id)
    }

    /// Gets a faction that matches this name (case sensitive).
    pub fn from_name(&self, name: &str) -> Option<&Faction> {
        self.0.values().find(|x| x.name == name)
    }

    /// Sets the relationship between a faction and another entity, with a potential faction.
    ///
    /// - *b*: If b is a faction id, attempts to set the relation with that faction. Otherwise
    ///   defaults to using `ent_id`
    ///
    /// - *ent_id*: Fallback if `b` is None or not a valid faction id. If this is `None`, nothing
    ///   will happen. Otherwise, this faction's relation will be adjusted for this specific entity.
    ///   
    /// Factions cannot be allies with a non-factioned entity, so the closest [`FactionRelation`] will be
    /// chosen that matches the given `relation`
    pub fn set_relation(&mut self, a: &FactionId, b: Option<&FactionId>, ent_id: Option<&EntityId>, relation: FactionRelation) {
        if let Some(b) = b {
            if let Some([a, b]) = self.0.get_many_mut([a, b]) {
                a.relationships.insert(b.id, relation);
                b.relationships.insert(a.id, relation);
                return;
            }
        }

        let Some(a) = self.0.get_mut(a) else {
            return;
        };

        if let Some(eid) = ent_id {
            if relation == FactionRelation::Enemy {
                a.at_war_with.push(*eid);
            } else {
                a.at_war_with.retain(|x| x != eid);
            }
        }
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
