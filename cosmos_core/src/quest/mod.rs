use std::num::NonZeroU32;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::{registry::sync_registry, sync_component, IdentifiableComponent, SyncableComponent},
    physics::location::Location,
    registry::{create_registry, identifiable::Identifiable},
};

#[derive(Default, Reflect, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct OngoingQuestDetails {
    pub payout: Option<NonZeroU32>,
    pub location: Option<Location>,
}

#[derive(Reflect, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct OngoingQuest {
    pub quest_id: u16,
    pub details: OngoingQuestDetails,
}

#[derive(Debug, Component, Reflect, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct OngoingQuests(Vec<OngoingQuest>);

impl OngoingQuests {
    pub fn start_quest(&mut self, quest: &Quest, details: OngoingQuestDetails) {
        self.0.push(OngoingQuest {
            quest_id: quest.id(),
            details,
        })
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
pub struct Quest {
    id: u16,
    unlocalized_name: String,
    description: String,
}

impl Quest {
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
