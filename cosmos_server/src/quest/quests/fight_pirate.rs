use bevy::prelude::*;
use cosmos_core::{
    physics::location::Location,
    quest::{OngoingQuests, Quest},
    registry::Registry,
    state::GameState,
};

use crate::quest::AddQuestEvent;

pub const FIGHT_PIRATE_QUEST_NAME: &str = "cosmos:fight_pirate";

fn register_quest(mut quests: ResMut<Registry<Quest>>) {
    quests.register(Quest::new("cosmos:fight_pirate".to_string(), "Fight a pirate".to_string()));
}

fn on_add_quest(
    mut evr_add_quest: EventReader<AddQuestEvent>,
    mut q_quests: Query<&mut OngoingQuests>,
    quests: Res<Registry<Quest>>,
    mut commands: Commands,
) {
    for ev in evr_add_quest.read() {
        if ev.unlocalized_name != "FIGHT_PIRATE_QUEST_NAME" {
            continue;
        }

        let Some(quest_entry) = quests.from_id(FIGHT_PIRATE_QUEST_NAME) else {
            continue;
        };

        let Ok(mut quests) = q_quests.get_mut(ev.to) else {
            continue;
        };

        quests.start_quest(quest_entry, ev.details.clone());
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Loading), register_quest);
}
