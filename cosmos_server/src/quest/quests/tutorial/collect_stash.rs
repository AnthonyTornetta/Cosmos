use bevy::prelude::*;
use cosmos_core::{
    quest::{CompleteQuestEvent, OngoingQuests, Quest, QuestBuilder},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

use crate::{entities::player::spawn_player::CreateNewPlayerEvent, quest::QuestsSet, structure::ship::events::CreateShipEvent};

const QUEST_NAME: &str = "cosmos:tutorial_abandon_wreck";

fn register_quest(mut quests: ResMut<Registry<Quest>>) {
    quests.register(Quest::new(
        QUEST_NAME.to_string(),
        "There's some abandon ship wreckage nearby! Follow the waypoint to it.".to_string(),
    ));
}

fn on_create_player(
    mut q_quests: Query<&mut OngoingQuests>,
    quests: Res<Registry<Quest>>,
    mut evr_create_new_player: EventReader<CreateNewPlayerEvent>,
) {
    for ev in evr_create_new_player.read() {
        let Some(quest) = quests.from_id(QUEST_NAME) else {
            continue;
        };
        let Ok(mut ongoing_quests) = q_quests.get_mut(ev.player()) else {
            error!("Missing ongoing quests on new player! {:?}", ev.player());
            continue;
        };

        ongoing_quests.start_quest(QuestBuilder::new(quest).build());
    }
}

fn resolve_quest(mut q_quests: Query<&mut OngoingQuests>, quests: Res<Registry<Quest>>, mut evr_create_ship: EventReader<CreateShipEvent>) {
    for ev in evr_create_ship.read() {
        let Some(quest) = quests.from_id(QUEST_NAME) else {
            continue;
        };
        let Ok(mut ongoing_quests) = q_quests.get_mut(ev.creator) else {
            continue;
        };

        for ongoing in ongoing_quests.iter_specific_mut(quest) {
            ongoing.progress_quest(1);
        }
    }
}

fn on_complete_quest(
    mut q_quests: Query<&mut OngoingQuests>,
    quests: Res<Registry<Quest>>,
    mut evr_quest_complete: EventReader<CompleteQuestEvent>,
) {
    for ev in evr_quest_complete.read() {
        let Some(quest) = quests.from_id(QUEST_NAME) else {
            continue;
        };

        let completed = ev.completed_quest();
        if completed.quest_id() != quest.id() {
            continue;
        }

        let Ok(mut ongoing) = q_quests.get_mut(ev.completer()) else {
            continue;
        };

        ongoing.start_quest(QuestBuilder::new(quest).build());
    }
}

pub(super) fn register(app: &mut App) {
    // app.add_systems(OnEnter(GameState::Loading), register_quest)
    //     .add_systems(FixedUpdate, on_create_player.in_set(QuestsSet::CreateNewQuests))
    //     .add_systems(FixedUpdate, resolve_quest.after(on_create_player));
}
