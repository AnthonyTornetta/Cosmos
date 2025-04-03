use bevy::prelude::*;
use cosmos_core::{
    entities::player::Player,
    quest::{OngoingQuestDetails, OngoingQuests},
};

mod quests;

#[derive(Event)]
pub struct AddQuestEvent {
    pub unlocalized_name: String,
    pub to: Entity,
    pub details: OngoingQuestDetails,
}

fn add_ongoing_quests(mut commands: Commands, q_players_no_quests: Query<Entity, (With<Player>, Without<OngoingQuests>)>) {
    for e in q_players_no_quests.iter() {
        commands.entity(e).insert(OngoingQuests::default());
    }
}

pub(super) fn register(app: &mut App) {
    quests::register(app);

    app.add_systems(Update, add_ongoing_quests);

    app.add_event::<AddQuestEvent>();
}
