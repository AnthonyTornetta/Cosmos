//! Shared quest logic

use std::num::{NonZero, NonZeroU32};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    item::Item,
    netty::sync::{
        IdentifiableComponent, SyncableComponent,
        events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
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

    // This field breaks the reflect derive, even though I wish this could be reflected :(
    #[reflect(ignore)]
    subquests: Option<OngoingQuests>,

    block_on: Vec<OngoingQuestId>,
}

/// Used to easily create new [`OngoingQuest`]s.
pub struct QuestBuilder {
    quest_id: u16,
    details: OngoingQuestDetails,
    starting_progress: u32,
    max_progress: Option<u32>,
    subquests: Option<OngoingQuests>,
    block_on: Vec<OngoingQuestId>,
}

impl QuestBuilder {
    /// Creates a new [`OngoingQuest`] builder with default values
    pub fn new(quest_type: &Quest) -> Self {
        Self {
            quest_id: quest_type.id(),
            subquests: None,
            max_progress: None,
            starting_progress: 0,
            details: Default::default(),
            block_on: vec![],
        }
    }

    /// Assigns the payout upon compelting this quest
    pub fn with_payout(mut self, amount: NonZero<u32>) -> Self {
        self.details.payout = Some(amount);
        self
    }

    /// Sets the location this quest takes place in. Useful for adding waypoints to the map for the
    /// player.
    pub fn with_location(mut self, location: Location) -> Self {
        self.details.location = Some(location);
        self
    }

    /// Assigns subquests as part of this quest.
    ///
    /// This quest will only be considered completed if all these subquests are complete and any
    /// other completion conditions are met.
    pub fn with_subquests(mut self, subquests: impl IntoIterator<Item = OngoingQuest>) -> Self {
        let subqs = self.subquests.get_or_insert_default();
        for q in subquests.into_iter() {
            subqs.start_quest(q);
        }
        self
    }

    /// TODO: This does nothing. Probably going to be removed
    pub fn depends_on_being_done(mut self, needs_done: &OngoingQuest) -> Self {
        self.block_on.push(needs_done.ongoing_id());
        self
    }

    /// Assigns the maximum progress this quest can have before it's marked for completion. Once
    /// progress == max_progress, this completion criteria will be met. If max_progress is 0, this
    /// completion criteria will always be true.
    pub fn with_max_progress(mut self, max_progress: u32) -> Self {
        self.max_progress = Some(max_progress);
        self
    }

    /// Assigns the starting progress this quest will have. This will be capped to the
    /// `max_progress`.
    pub fn with_starting_progress(mut self, starting_progress: u32) -> Self {
        self.starting_progress = starting_progress;
        self
    }

    /// Creates the [`OngoingQuest`] instance ready to be used.
    pub fn build(self) -> OngoingQuest {
        let max_progress = self.max_progress.unwrap_or_else(|| {
            if self.subquests.as_ref().is_some_and(|sq| !sq.0.is_empty()) {
                0
            } else {
                1
            }
        });

        OngoingQuest::new_raw(
            self.quest_id,
            self.details,
            self.starting_progress.min(max_progress),
            max_progress,
            self.subquests,
            self.block_on,
        )
    }
}

#[derive(Reflect, Debug, Serialize, Deserialize, Clone, PartialEq, Copy, Eq, Hash)]
/// A unique ID that can be used to find an ongoing quest that a player has
pub struct OngoingQuestId(Uuid);

#[derive(Debug, Component, Reflect, Serialize, Deserialize, Clone, PartialEq, Default)]
/// All quests that this entity currently has ongoing
pub struct OngoingQuests(Vec<OngoingQuest>);

impl OngoingQuest {
    /// Creates a new ongoing quest with these details. You probably want to use the [`QuestBuilder`] instead.
    pub fn new(quest_type: &Quest, details: OngoingQuestDetails, max_progress: u32) -> Self {
        let id = OngoingQuestId(Uuid::new_v4());

        Self {
            ongoing_id: id,
            quest_id: quest_type.id(),
            subquests: None,
            details,
            progress: 0,
            max_progress,
            block_on: vec![],
        }
    }

    fn new_raw(
        quest_id: u16,
        details: OngoingQuestDetails,
        progress: u32,
        max_progress: u32,
        subquests: Option<OngoingQuests>,
        block_on: Vec<OngoingQuestId>,
    ) -> Self {
        let id = OngoingQuestId(Uuid::new_v4());

        Self {
            ongoing_id: id,
            quest_id,
            subquests,
            details,
            progress,
            max_progress,
            block_on,
        }
    }

    /// Returns the maximum progress this quest can have
    pub fn max_progress(&self) -> u32 {
        self.max_progress
    }

    /// Returns how much progress has been made on this quest so far. Will always be <=
    /// [`Self::max_progress`]
    pub fn progress(&self) -> u32 {
        self.progress
    }

    /// Adds a subquest to this ongoing quest. This quest can only be marked as complete if all
    /// subquests are complete too.
    pub fn add_subquest(&mut self, subquest: OngoingQuest) -> &mut Self {
        self.subquests.get_or_insert_default().start_quest(subquest);
        self
    }

    /// Returns the id for the [`Quest`] this is an instance of. Get the [`Quest`] via
    /// [`Registry<Quest>`].
    pub fn quest_id(&self) -> u16 {
        self.quest_id
    }

    /// Returns this quest's unique ID, to be used with the [`OngoingQuests`] component
    pub fn ongoing_id(&self) -> OngoingQuestId {
        self.ongoing_id
    }

    /// Sets the progress of this quest - capped at `max_progress`
    pub fn set_progress(&mut self, progress: u32) -> bool {
        if progress >= self.max_progress {
            self.progress = self.max_progress;
            true
        } else {
            self.progress = progress;
            false
        }
    }

    /// Adds this amount of progress to the quest
    pub fn progress_quest(&mut self, progress: u32) -> bool {
        self.set_progress(self.progress + progress)
    }

    /// Completes this quest by setting this quest's progress to its maximum value.
    ///
    /// # IMPORTANT NOTE
    ///
    /// [`Self::completed`] can still return `false` after calling this if any of the subquests are
    /// not compelte.
    pub fn complete(&mut self) {
        self.progress = self.max_progress;
    }

    /// Returns all subquests this has (if any)
    pub fn subquests(&self) -> Option<&OngoingQuests> {
        self.subquests.as_ref()
    }

    /// Returns all subquests this has (if any)
    pub fn subquests_mut(&mut self) -> Option<&mut OngoingQuests> {
        self.subquests.as_mut()
    }

    /// Checks if this quest is completed (all its completion criteria are met).
    pub fn completed(&self) -> bool {
        self.subquests.as_ref().map(|x| x.iter().all(|q| q.completed())).unwrap_or(true) && self.progress == self.max_progress
    }
}

impl OngoingQuests {
    /// Starts a quest with the given details.
    ///
    /// Returns the [`OngoingQuestId`] for this quest for convenience
    pub fn start_quest(&mut self, ongoing_quest: OngoingQuest) -> OngoingQuestId {
        let q_id = ongoing_quest.ongoing_id();
        self.0.push(ongoing_quest);
        q_id
    }

    /// Checks if this contains any [`OngoingQuest`]s of this [`Quest`] type.
    pub fn contains(&self, quest: &Quest) -> bool {
        self.0.iter().any(|ongoing| ongoing.quest_id() == quest.id())
    }

    /// Checks if this contains any [`OngoingQuest`]s of this [`Quest`] type.
    pub fn contains_ongoing(&self, quest: &OngoingQuestId) -> bool {
        self.0.iter().any(|ongoing| ongoing.ongoing_id() == *quest)
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

    /// Returns the first instance of this quest if one exists
    pub fn get_quest(&self, quest: &Quest) -> Option<&OngoingQuest> {
        self.iter_specific(quest).next()
    }

    /// Returns the first instance of this quest if one exists
    pub fn get_quest_mut(&mut self, quest: &Quest) -> Option<&mut OngoingQuest> {
        self.iter_specific_mut(quest).next()
    }

    /// Iterates over all [`OngoingQuest`]s.
    pub fn iter(&self) -> impl Iterator<Item = &'_ OngoingQuest> {
        self.0.iter()
    }

    /// Iterates over [`OngoingQuest`]s of this [`Quest`] type.
    pub fn iter_specific(&self, quest: &Quest) -> impl Iterator<Item = &'_ OngoingQuest> {
        self.iter().filter(|q| q.quest_id == quest.id())
    }

    /// Iterates over all [`OngoingQuest`]s.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &'_ mut OngoingQuest> {
        self.0.iter_mut()
    }

    /// Iterates over [`OngoingQuest`]s of this [`Quest`] type.
    ///
    /// # WARNING: This triggers change detection, even if the quest isn't found!
    /// Please use [`Self::contains`] first!
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
    /// An item's id
    icon: Option<u16>,
}

impl Quest {
    /// Creates a new type of quest.
    pub fn new(unlocalized_name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: 0,
            unlocalized_name: unlocalized_name.into(),
            description: description.into(),
            icon: None,
        }
    }

    /// Creates a quest type that will show up with an item icon
    pub fn new_with_icon(unlocalized_name: impl Into<String>, description: impl Into<String>, icon: &Item) -> Self {
        Self {
            id: 0,
            unlocalized_name: unlocalized_name.into(),
            description: description.into(),
            icon: Some(icon.id()),
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

#[derive(Message, Serialize, Deserialize, Clone, Debug)]
/// Sent whenever the player completes an [`OngoingQuest`].
pub struct CompleteQuestMessage {
    completer: Entity,
    completed_quest: OngoingQuest,
}

impl CompleteQuestMessage {
    /// Creates a new quest complete event
    ///
    /// - `completer` - The entity that completed the quest (Should have the [`OngoingQuests`]
    ///   component).
    /// - `completed` - The [`OngoingQuest`] they completed.
    pub fn new(completer: Entity, completed: OngoingQuest) -> Self {
        Self {
            completer,
            completed_quest: completed,
        }
    }

    /// The quest that was completed
    pub fn completed_quest(&self) -> &OngoingQuest {
        &self.completed_quest
    }

    /// Who completed the quest
    ///
    /// This entity should have an [`OngoingQuests`] component.
    pub fn completer(&self) -> Entity {
        self.completer
    }
}

impl IdentifiableMessage for CompleteQuestMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:complete_quest"
    }
}

impl NettyMessage for CompleteQuestMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
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

#[derive(Component, Debug, Reflect, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
/// The player will have this if they currently have this quest selected.
///
/// This does NOT mean they can only do this quest - rather they have this one focused on at the
/// moment for information reasons. We should display information relevant to this quest in places
/// that make sense.
pub struct ActiveQuest(pub OngoingQuestId);

impl IdentifiableComponent for ActiveQuest {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:active_quest"
    }
}

impl SyncableComponent for ActiveQuest {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Message, Serialize, Deserialize, Clone, Debug)]
/// Sent by the client to set their active quest
pub struct SetActiveQuestMessage {
    /// The quest they want to make their active quest. Must be in their [`OngoingQuests`]
    /// componment.
    pub quest: Option<OngoingQuestId>,
}

impl IdentifiableMessage for SetActiveQuestMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:set_active_quest"
    }
}

impl NettyMessage for SetActiveQuestMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    create_registry::<Quest>(app, "cosmos:quest");
    sync_registry::<Quest>(app);

    sync_component::<ActiveQuest>(app);
    sync_component::<OngoingQuests>(app);

    app.register_type::<OngoingQuests>()
        .register_type::<ActiveQuest>()
        .add_netty_event::<CompleteQuestMessage>()
        .add_netty_event::<SetActiveQuestMessage>();
}
