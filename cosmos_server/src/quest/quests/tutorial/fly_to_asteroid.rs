use bevy::prelude::*;
use cosmos_core::{
    physics::location::Location,
    prelude::Asteroid,
    quest::{ActiveQuest, OngoingQuests, Quest, QuestBuilder},
    registry::Registry,
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
    mut commands: Commands,
    mut q_quests: Query<
        (Entity, &mut OngoingQuests, &TutorialState),
        Or<(Changed<TutorialState>, (Added<OngoingQuests>, With<TutorialState>))>,
    >,
    quests: Res<Registry<Quest>>,
) {
    for (ent, mut ongoing_quests, tutorial_state) in q_quests.iter_mut() {
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

        let q_id = ongoing_quests.start_quest(main_quest);
        commands.entity(ent).insert(ActiveQuest(q_id));
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
        if !ongoing_quests.contains(quest) {
            continue;
        }

        if q_asteroid
            .iter()
            .any(|l| l.is_within_reasonable_range(loc) && l.distance_sqrd(loc) < 500.0 * 500.0)
        {
            // Avoids change detection unless we are 100% changing it by doing this get here
            let Some(ongoing) = ongoing_quests.get_quest_mut(quest) else {
                continue;
            };

            ongoing.complete();
        }
    }
}

pub(super) fn register(app: &mut App) {
    super::add_tutorial(app, MAIN_QUEST_NAME);

    app.add_systems(OnEnter(GameState::PostLoading), register_quest).add_systems(
        FixedUpdate,
        (
            on_change_tutorial_state.in_set(QuestsSet::CreateNewQuests),
            resolve_quests.after(on_change_tutorial_state).before(QuestsSet::CompleteQuests),
        ),
    );
}
