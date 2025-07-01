//! Shared quest logic

use std::num::NonZeroU32;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    netty::sync::{
        IdentifiableComponent, SyncableComponent,
        events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
        registry::sync_registry,
        sync_component,
    },
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
    quest_id: u16,
    /// The details for this quest
    pub details: OngoingQuestDetails,
    progress: u32,
    max_progress: u32,
    ongoing_id: OngoingQuestId,
}

#[derive(Reflect, Debug, Serialize, Deserialize, Clone, PartialEq, Copy)]
/// A unique ID that can be used to find an ongoing quest that a player has
pub struct OngoingQuestId(Uuid);

#[derive(Debug, Component, Reflect, Serialize, Deserialize, Clone, PartialEq, Default)]
/// All quests that this entity currently has ongoing
pub struct OngoingQuests(Vec<OngoingQuest>);

impl OngoingQuest {
    pub fn quest_id(&self) -> u16 {
        self.quest_id
    }

    /// Returns this quest's unique ID, to be used with the [`OngoingQuests`] component
    pub fn ongoing_id(&self) -> OngoingQuestId {
        self.ongoing_id
    }

    pub fn progress_quest(&mut self, progress: u32) -> bool {
        if self.progress + progress >= self.max_progress {
            self.progress = self.max_progress;
            true
        } else {
            self.progress += progress;
            false
        }
    }

    pub fn completed(&self) -> bool {
        self.progress == self.max_progress
    }
}

impl OngoingQuests {
    /// Starts a quest with the given details.
    pub fn start_quest(&mut self, quest: &Quest, details: OngoingQuestDetails, max_progress: u32) -> OngoingQuestId {
        let id = OngoingQuestId(Uuid::new_v4());
        self.0.push(OngoingQuest {
            quest_id: quest.id(),
            details,
            ongoing_id: id,
            progress: 0,
            max_progress,
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

    /// Returns `None` if this quest id doesn't exist.
    ///
    /// Returns `Some(true)` if this completes the quest, `Some(false)` otherwise.
    pub fn progress_quest(&mut self, id: &OngoingQuestId, progress: u32) -> Option<bool> {
        self.0
            .iter_mut()
            .find(|quest| quest.ongoing_id == *id)
            .map(|quest| quest.progress_quest(progress))
    }

    /// Iterates over all ongoing quests
    pub fn iter(&self) -> impl Iterator<Item = &'_ OngoingQuest> {
        self.0.iter()
    }

    pub fn iter_specific(&self, quest: &Quest) -> impl Iterator<Item = &'_ OngoingQuest> {
        self.iter().filter(|q| q.quest_id == quest.id())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &'_ mut OngoingQuest> {
        self.0.iter_mut()
    }

    pub fn iter_specific_mut(&mut self, quest: &Quest) -> impl Iterator<Item = &'_ mut OngoingQuest> {
        self.iter_mut().filter(|q| q.quest_id == quest.id())
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

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct CompleteQuestEvent {
    completer: Entity,
    completed_quest: OngoingQuest,
}

impl CompleteQuestEvent {
    pub fn new(completer: Entity, completed: OngoingQuest) -> Self {
        Self {
            completer,
            completed_quest: completed,
        }
    }

    pub fn completed_quest(&self) -> &OngoingQuest {
        &self.completed_quest
    }

    pub fn completer(&self) -> Entity {
        self.completer
    }
}

impl IdentifiableEvent for CompleteQuestEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:complete_quest"
    }
}

impl NettyEvent for CompleteQuestEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.client_from_server(&self.completer).map(|e| Self {
            completer: e,
            completed_quest: self.completed_quest,
        })
    }
}

pub(super) fn register(app: &mut App) {
    create_registry::<Quest>(app, "cosmos:quest");
    sync_registry::<Quest>(app);

    sync_component::<OngoingQuests>(app);

    app.register_type::<OngoingQuests>();

    app.add_netty_event::<CompleteQuestEvent>();
}
