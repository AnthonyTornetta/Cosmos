use bevy::prelude::*;
use cosmos_core::{
    quest::{CompleteQuestEvent, OngoingQuest, OngoingQuestDetails, OngoingQuests, Quest},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

use crate::{quest::QuestsSet, structure::ship::events::CreateShipEvent};

use super::TutorialState;

const QUEST_NAME: &str = "cosmos:tutorial_create_ship";

fn register_quest(mut quests: ResMut<Registry<Quest>>) {
    quests.register(Quest::new(
        QUEST_NAME.to_string(),
        "Press `x` while a ship core is in your inventory.".to_string(),
    ));
}

fn on_change_tutorial_state(
    mut q_quests: Query<(&mut OngoingQuests, &TutorialState), Or<(Changed<TutorialState>, (Added<OngoingQuests>, With<TutorialState>))>>,
    quests: Res<Registry<Quest>>,
) {
    for (mut ongoing_quests, tutorial_state) in q_quests.iter_mut() {
        if *tutorial_state != TutorialState::CreateShip {
            continue;
        }

        let Some(quest) = quests.from_id(QUEST_NAME) else {
            continue;
        };

        if ongoing_quests.contains(quest) {
            continue;
        }

        info!("ADDING QUEST: {quest:?}");

        ongoing_quests.start_quest(OngoingQuest::new(quest, OngoingQuestDetails { ..Default::default() }, 1));
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
            ongoing.complete();
        }
    }
}

fn on_complete_quest(
    mut q_tutorial_state: Query<&mut TutorialState>,
    quests: Res<Registry<Quest>>,
    mut evr_quest_complete: EventReader<CompleteQuestEvent>,
    mut commands: Commands,
) {
    for ev in evr_quest_complete.read() {
        info!("Got Event!");
        let Some(quest) = quests.from_id(QUEST_NAME) else {
            info!("Bad Quest Event!");
            continue;
        };

        let completed = ev.completed_quest();
        if completed.quest_id() != quest.id() {
            info!("Bad Quest ID {quest:?} {completed:?}!");
            continue;
        }

        let Ok(mut tutorial_state) = q_tutorial_state.get_mut(ev.completer()) else {
            info!("Bad TS!");
            continue;
        };

        if let Some(state) = tutorial_state.next_state() {
            info!("Change to {state:?}");
            *tutorial_state = state;
        } else {
            commands.entity(ev.completer()).remove::<TutorialState>();
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), register_quest).add_systems(
        FixedUpdate,
        (
            on_change_tutorial_state.in_set(QuestsSet::CreateNewQuests),
            resolve_quest.after(on_change_tutorial_state).before(QuestsSet::CompleteQuests),
            on_complete_quest.after(QuestsSet::CompleteQuests),
        ),
    );
}
