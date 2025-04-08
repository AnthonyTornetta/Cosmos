//! Server quest logic

use bevy::prelude::*;
use cosmos_core::{
    entities::player::Player,
    quest::{OngoingQuestDetails, OngoingQuests},
};

use crate::persistence::make_persistent::{make_persistent, DefaultPersistentComponent};

mod quests;

#[derive(Event)]
/// Is this needed?
///
/// Send this event to add a new quest
pub struct AddQuestEvent {
    /// The unlocalized name of the quest
    pub unlocalized_name: String,
    /// The player entity that should get this quest
    pub to: Entity,
    /// The details for this quest
    /// TODO: this is stupid
    pub details: OngoingQuestDetails,
}

fn add_ongoing_quests(mut commands: Commands, q_players_no_quests: Query<Entity, (With<Player>, Without<OngoingQuests>)>) {
    for e in q_players_no_quests.iter() {
        commands.entity(e).insert(OngoingQuests::default());
    }
}

impl DefaultPersistentComponent for OngoingQuests {}

pub(super) fn register(app: &mut App) {
    quests::register(app);

    make_persistent::<OngoingQuests>(app);

    app.add_systems(Update, add_ongoing_quests);

    app.add_event::<AddQuestEvent>();
}
