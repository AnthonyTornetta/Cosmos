use bevy::prelude::*;
use cosmos_core::{
    physics::location::Location,
    prelude::Asteroid,
    quest::{CompleteQuestEvent, OngoingQuests, Quest, QuestBuilder},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

use crate::quest::QuestsSet;

use super::TutorialState;

const MAIN_QUEST_NAME: &str = "cosmos:tutorial_fly_to_asteroid";

fn register_quest(mut quests: ResMut<Registry<Quest>>) {
    quests.register(Quest::new(
        MAIN_QUEST_NAME.to_string(),
        "Fly to an asteroid (one of the brown indicators)".to_string(),
    ));
}

fn on_change_tutorial_state(
    mut q_quests: Query<(&mut OngoingQuests, &TutorialState), Or<(Changed<TutorialState>, (Added<OngoingQuests>, With<TutorialState>))>>,
    quests: Res<Registry<Quest>>,
) {
    for (mut ongoing_quests, tutorial_state) in q_quests.iter_mut() {
        if *tutorial_state != TutorialState::FlyToAsteroid {
            continue;
        }

        let Some(main_quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        if ongoing_quests.contains(main_quest) {
            continue;
        }

        let main_quest = QuestBuilder::new(main_quest).build();

        ongoing_quests.start_quest(main_quest);
    }
}

fn resolve_quests(
    quests: Res<Registry<Quest>>,
    mut q_ongoing_quests: Query<(&Location, &mut OngoingQuests)>,
    q_asteroid: Query<&Location, With<Asteroid>>,
) {
    let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
        return;
    };

    for (loc, mut ongoing_quests) in q_ongoing_quests.iter_mut() {
        let Some(ongoing) = ongoing_quests.get_quest_mut(quest) else {
            continue;
        };

        if q_asteroid
            .iter()
            .any(|l| l.is_within_reasonable_range(loc) && l.distance_sqrd(loc) < 500.0 * 500.0)
        {
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
        let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        let completed = ev.completed_quest();
        if completed.quest_id() != quest.id() {
            continue;
        }

        let Ok(mut tutorial_state) = q_tutorial_state.get_mut(ev.completer()) else {
            continue;
        };

        if let Some(state) = tutorial_state.next_state() {
            info!("Advancing tutorital state to {state:?}");
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
            resolve_quests.after(on_change_tutorial_state).before(QuestsSet::CompleteQuests),
            on_complete_quest.after(QuestsSet::CompleteQuests),
        ),
    );
}
