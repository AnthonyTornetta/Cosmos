use bevy::prelude::*;
use cosmos_core::{
    quest::{ActiveQuest, OngoingQuest, OngoingQuestDetails, OngoingQuests, Quest},
    registry::Registry,
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
    mut q_quests: Query<
        (Entity, &mut OngoingQuests, &TutorialState),
        Or<(Changed<TutorialState>, (Added<OngoingQuests>, With<TutorialState>))>,
    >,
    quests: Res<Registry<Quest>>,
    mut commands: Commands,
) {
    for (ent, mut ongoing_quests, tutorial_state) in q_quests.iter_mut() {
        if *tutorial_state != TutorialState::CreateShip {
            continue;
        }

        let Some(quest) = quests.from_id(QUEST_NAME) else {
            continue;
        };

        if ongoing_quests.contains(quest) {
            continue;
        }

        let q_id = ongoing_quests.start_quest(OngoingQuest::new(quest, OngoingQuestDetails { ..Default::default() }, 1));
        commands.entity(ent).insert(ActiveQuest(q_id));
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

        if !ongoing_quests.contains(quest) {
            continue;
        }

        for ongoing in ongoing_quests.iter_specific_mut(quest) {
            ongoing.complete();
        }
    }
}

pub(super) fn register(app: &mut App) {
    super::add_tutorial(app, QUEST_NAME);

    app.add_systems(OnEnter(GameState::PostLoading), register_quest).add_systems(
        FixedUpdate,
        (
            on_change_tutorial_state.in_set(QuestsSet::CreateNewQuests),
            resolve_quest.after(on_change_tutorial_state).before(QuestsSet::CompleteQuests),
        ),
    );
}
