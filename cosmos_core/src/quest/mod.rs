//! Shared quest logic

use std::num::NonZeroU32;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    netty::sync::{registry::sync_registry, sync_component, IdentifiableComponent, SyncableComponent},
    physics::location::Location,
    registry::{create_registry, identifiable::Identifiable},
};

#[derive(Default, Reflect, Debug, Serialize, Deserialize, Clone, PartialEq)]
/// The details about a quest a player is on. Will be stored in the [`OngoingQuests`] component.
///
/// TODO: Refactor this stuff.
pub struct OngoingQuestDetails {
    /// How much money will be made upon completion of quest (if any)
    pub payout: Option<NonZeroU32>,
    /// The location the quest happens (if any)
    pub location: Option<Location>,
}

#[derive(Reflect, Debug, Serialize, Deserialize, Clone, PartialEq)]
/// A quest that the player has signed up to do, and not yet completed.
pub struct OngoingQuest {
    /// The quest id, referencing the [`Registry<Quest>`].
    pub quest_id: u16,
    /// The details for this quest
    pub details: OngoingQuestDetails,
    ongoing_id: OngoingQuestId,
}

#[derive(Reflect, Debug, Serialize, Deserialize, Clone, PartialEq, Copy)]
/// A unique ID that can be used to find an ongoing quest that a player has
pub struct OngoingQuestId(Uuid);

#[derive(Debug, Component, Reflect, Serialize, Deserialize, Clone, PartialEq, Default)]
/// All quests that this entity currently has ongoing
pub struct OngoingQuests(Vec<OngoingQuest>);

impl OngoingQuest {
    /// Returns this quest's unique ID, to be used with the [`OngoingQuests`] component
    pub fn ongoing_id(&self) -> OngoingQuestId {
        self.ongoing_id
    }
}

impl OngoingQuests {
    /// Iterates over all ongoing quests
    pub fn iter(&self) -> impl Iterator<Item = &OngoingQuest> {
        self.0.iter()
    }

    /// Starts a quest with the given details.
    pub fn start_quest(&mut self, quest: &Quest, details: OngoingQuestDetails) -> OngoingQuestId {
        let id = OngoingQuestId(Uuid::new_v4());
        self.0.push(OngoingQuest {
            quest_id: quest.id(),
            details,
            ongoing_id: id,
        });

        id
    }

    /// Gets an ongoing quest from its id, if one exists
    pub fn from_id(&self, id: &OngoingQuestId) -> Option<&OngoingQuest> {
        self.0.iter().find(|x| x.ongoing_id == *id)
    }

    /// Removes an ongoing quest from its id, if one exists
    pub fn remove_ongoing_quest(&mut self, id: &OngoingQuestId) -> Option<OngoingQuest> {
        let (idx, _) = self.0.iter().enumerate().find(|(_, x)| x.ongoing_id == *id)?;

        Some(self.0.remove(idx))
    }
}

impl IdentifiableComponent for OngoingQuests {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:ongoing_quests"
    }
}

impl SyncableComponent for OngoingQuests {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Something the player needs to accomplish for a reward, ususally given by NPCs
/// This describes the type of quest, use [`OngoingQuest`] for quests a player is currently on.
///
/// Use the [`OngoingQuests`] component present on players to start/end quests
pub struct Quest {
    id: u16,
    unlocalized_name: String,
    /// TODO: Encode this in some sort of registry loaded from a lang file
    pub description: String,
}

impl Quest {
    /// Creates a new type of quest.
    pub fn new(unlocalized_name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: 0,
            unlocalized_name: unlocalized_name.into(),
            description: description.into(),
        }
    }
}

impl Identifiable for Quest {
    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }

    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }
}

pub(super) fn register(app: &mut App) {
    create_registry::<Quest>(app, "cosmos:quest");
    sync_registry::<Quest>(app);

    sync_component::<OngoingQuests>(app);

    app.register_type::<OngoingQuests>();
}
