use bevy::prelude::*;

use crate::{entities::player::spawn_player::CreateNewPlayerEvent, quest::QuestsSet};

mod collect_stash;
mod create_a_ship;
mod fly_a_ship;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
enum TutorialState {
    CreateShip,
    LearnToFly,
    CollectStash,
    BuildShip,
    FlyToAsteroid,
    MineAsteroid,
    Craft,
    Fight,
}

impl TutorialState {
    pub fn next_state(&self) -> Option<Self> {
        use TutorialState as S;
        match self {
            S::CreateShip => Some(S::LearnToFly),
            S::LearnToFly => Some(S::CollectStash),
            S::CollectStash => Some(S::BuildShip),
            S::BuildShip => Some(S::FlyToAsteroid),
            S::FlyToAsteroid => Some(S::MineAsteroid),
            S::MineAsteroid => Some(S::Craft),
            S::Craft => Some(S::Fight),
            S::Fight => None,
        }
    }
}

fn on_create_player(mut commands: Commands, mut evr_create_new_player: EventReader<CreateNewPlayerEvent>) {
    for ev in evr_create_new_player.read() {
        commands.entity(ev.player()).try_insert(TutorialState::CollectStash);
    }
}

// fn add_tutorial_quest(app: &mut App, tutorial_state: TutorialState, quest_identifier: &'static str, quest_description: &) {
//     let register_quest = |mut quests: ResMut<Registry<Quest>>| {
//         quests.register(Quest::new(quest_identifier.to_string(), "Learn to fly your ship".to_string()));
//     };
//
//     fn on_change_tutorial_state(
//         mut q_quests: Query<
//             (&mut OngoingQuests, &TutorialState),
//             Or<(Changed<TutorialState>, (Added<OngoingQuests>, With<TutorialState>))>,
//         >,
//         quests: Res<Registry<Quest>>,
//     ) {
//         for (mut ongoing_quests, tutorial_state) in q_quests.iter_mut() {
//             if *tutorial_state != TutorialState::LearnToFly {
//                 continue;
//             }
//
//             let Some(quest) = quests.from_id(QUEST_NAME) else {
//                 continue;
//             };
//
//             ongoing_quests.start_quest(quest, OngoingQuestDetails { ..Default::default() }, 1);
//         }
//     }
//
//     fn on_complete_quest(
//         mut q_tutorial_state: Query<&mut TutorialState>,
//         quests: Res<Registry<Quest>>,
//         mut evr_quest_complete: EventReader<CompleteQuestEvent>,
//         mut commands: Commands,
//     ) {
//         for ev in evr_quest_complete.read() {
//             let Some(quest) = quests.from_id(QUEST_NAME) else {
//                 continue;
//             };
//
//             let completed = ev.completed_quest();
//             if completed.quest_id() != quest.id() {
//                 continue;
//             }
//
//             let Ok(mut tutorial_state) = q_tutorial_state.get_mut(ev.completer()) else {
//                 continue;
//             };
//
//             if let Some(state) = tutorial_state.next_state() {
//                 *tutorial_state = state;
//             } else {
//                 commands.entity(ev.completer()).remove::<TutorialState>();
//             }
//         }
//     }
//
//     pub(super) fn register(app: &mut App) {
//         app.add_systems(OnEnter(GameState::Loading), register_quest).add_systems(
//             FixedUpdate,
//             (
//                 on_change_tutorial_state.in_set(QuestsSet::CreateNewQuests),
//                 resolve_quest.after(on_change_tutorial_state).before(QuestsSet::CompleteQuests),
//                 on_complete_quest.after(QuestsSet::CompleteQuests),
//             ),
//         );
//     }
// }

pub(super) fn register(app: &mut App) {
    app.add_systems(FixedUpdate, on_create_player.in_set(QuestsSet::CreateNewQuests))
        .register_type::<TutorialState>();

    create_a_ship::register(app);
    collect_stash::register(app);
    fly_a_ship::register(app);
}
