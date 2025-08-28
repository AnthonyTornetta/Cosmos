//! Factions

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entities::{EntityId, player::Player},
    netty::sync::{
        IdentifiableComponent, SyncableComponent,
        resources::{SyncableResource, sync_resource},
        sync_component,
    },
};

pub mod events;

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, Default)]
/// Extra configuration for a faction
pub struct FactionSettings {
    /// If this is true, this faction will automatically be at war with any neutral faction.
    pub default_enemy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
/// A player in a faction.
///
/// Stores basic information about the player that may be out of date, such as their name.
pub struct FactionPlayer {
    /// The player's entity id
    pub entity_id: EntityId,
    /// This name may be out of date, but good enough for easy display
    pub name: String,
}

impl FactionPlayer {
    /// Creates a new faction player referring to this player. Please make sure this entity id
    /// matches this player
    pub fn new(entity_id: EntityId, player: &Player) -> Self {
        Self {
            entity_id,
            name: player.name().to_owned(),
        }
    }

    /// Returns the cached name of this player
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
/// A collection of players/NPCs under a common team
///
/// Not every player will have a faction.
pub struct Faction {
    id: FactionId,
    name: String,
    players: Vec<FactionPlayer>,
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
        players: Vec<FactionPlayer>,
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
    pub fn id(&self) -> FactionId {
        self.id
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

    /// Returns the player-chosen name of this faction
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Adds a player to this faction
    pub fn add_player(&mut self, player: FactionPlayer) {
        if !self.players.iter().any(|x| x.entity_id == player.entity_id) {
            self.players.push(player);
        }
    }

    /// Removes a player from this faction if they are in it
    pub fn remove_player(&mut self, player_id: EntityId) {
        if let Some((idx, _)) = self.players.iter().enumerate().find(|(_, x)| x.entity_id == player_id) {
            self.players.remove(idx);
        }
    }

    /// Iterates over all players in this faction
    pub fn players(&self) -> impl Iterator<Item = &FactionPlayer> {
        self.players.iter()
    }

    /// Checks if this faction has no players. AI only factions will count as empty.
    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
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
    pub fn add_new_faction(&mut self, faction: Faction) -> bool {
        if self.0.contains_key(&faction.id) {
            return false;
        }
        if self.0.values().any(|x| x.name == faction.name) {
            return false;
        }
        self.0.insert(faction.id, faction);
        true
    }

    /// Gets a faction from this id
    pub fn from_id(&self, id: &FactionId) -> Option<&Faction> {
        self.0.get(id)
    }

    /// Gets a faction from this id
    pub fn from_id_mut(&mut self, id: &FactionId) -> Option<&mut Faction> {
        self.0.get_mut(id)
    }

    /// Removes a faction from the game.
    pub fn remove_faction(&mut self, id: &FactionId) -> Option<Faction> {
        self.0.remove(id)
    }

    /// Returns if there already contains a faction with a name similar to this. This does NOT mean
    /// [`Self::from_name`] will return a faction for this valid.
    pub fn is_name_unique(&self, name: &str) -> bool {
        let stripped = name.replace(" ", "").to_lowercase();
        !self.0.values().any(|x| x.name.replace(" ", "").to_lowercase() == stripped)
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
        if let Some(b) = b
            && let [Some(a), Some(b)] = self.0.get_many_mut([a, b])
        {
            a.relationships.insert(b.id, relation);
            b.relationships.insert(a.id, relation);
            return;
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

#[derive(Component, Clone, Debug, Reflect, Serialize, Deserialize, PartialEq, Eq, Default)]
/// A list of factions this player has been invited to
///
/// This will be cleared when the player disconnects
pub struct FactionInvites(HashSet<FactionId>);

impl FactionInvites {
    /// Creates a [`FactionInvites`] list with an invite to this faction
    pub fn with_invite(faction: FactionId) -> Self {
        let mut s = Self::default();
        s.add_invite(faction);
        s
    }

    /// Checks if this contains an invite to this faction
    pub fn contains(&self, faction: FactionId) -> bool {
        self.0.contains(&faction)
    }

    /// Adds an invite to this faction (or does nothing if one already exists)
    pub fn add_invite(&mut self, faction: FactionId) {
        self.0.insert(faction);
    }

    /// Removes the invite to this faction (or does nothing if none exist)
    pub fn remove_invite(&mut self, faction: FactionId) {
        self.0.remove(&faction);
    }

    /// Iterates over all factions they have been invited to
    ///
    /// Some of these may no longer be valid IDs, so make sure to properly handle that case
    pub fn iter(&self) -> impl Iterator<Item = FactionId> {
        self.0.iter().copied()
    }

    /// Checks if there are no invites (valid or invalid).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IdentifiableComponent for FactionInvites {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:faction_invites"
    }
}

impl SyncableComponent for FactionInvites {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

pub(super) fn register(app: &mut App) {
    events::register(app);

    sync_resource::<Factions>(app);
    sync_component::<FactionId>(app);
    sync_component::<FactionInvites>(app);

    app.register_type::<FactionRelation>()
        .register_type::<Faction>()
        .register_type::<Uuid>()
        .register_type::<FactionId>()
        .register_type::<FactionInvites>();
}
