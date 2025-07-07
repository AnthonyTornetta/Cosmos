use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    netty::sync::IdentifiableComponent,
    prelude::Ship,
    quest::{CompleteQuestEvent, OngoingQuests, Quest, QuestBuilder},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::ship::{pilot::Pilot, ship_movement::ShipMovement},
};
use serde::{Deserialize, Serialize};

use crate::{
    persistence::make_persistent::{DefaultPersistentComponent, make_persistent},
    quest::QuestsSet,
};

use super::TutorialState;

const MAIN_QUEST_NAME: &str = "cosmos:tutorial_fly_a_ship";
const ENTER_SHIP_QUEST: &str = "cosmos:tutorial_enter_ship";
const MOVE_SHIP_QUEST: &str = "cosmos:tutorial_move_ship";
const ROTATE_SHIP_QUEST: &str = "cosmos:tutorial_rotate_ship";

fn register_quest(mut quests: ResMut<Registry<Quest>>) {
    quests.register(Quest::new(MAIN_QUEST_NAME.to_string(), "Learn to fly your ship.".to_string()));
    quests.register(Quest::new(ENTER_SHIP_QUEST.to_string(), "Use <R> to enter the ship.".to_string()));
    quests.register(Quest::new(
        MOVE_SHIP_QUEST.to_string(),
        "Use WASDEQ to move your ship. Hold <Shift> to brake.".to_string(),
    ));
    quests.register(Quest::new(
        ROTATE_SHIP_QUEST.to_string(),
        "Use your mouse and ZC to rotate your ship.".to_string(),
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
        if *tutorial_state != TutorialState::LearnToFly {
            continue;
        }

        let Some(main_quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        if ongoing_quests.contains(main_quest) {
            continue;
        }

        let Some(enter_ship) = quests.from_id(ENTER_SHIP_QUEST) else {
            continue;
        };
        let Some(move_ship) = quests.from_id(MOVE_SHIP_QUEST) else {
            continue;
        };
        let Some(rotate_ship) = quests.from_id(ROTATE_SHIP_QUEST) else {
            continue;
        };

        let enter_ship_quest = QuestBuilder::new(enter_ship).build();
        let move_ship_quest = QuestBuilder::new(move_ship)
            .with_max_progress(100)
            .depends_on_being_done(&enter_ship_quest)
            .build();
        let rotate_ship_quest = QuestBuilder::new(rotate_ship)
            .with_max_progress(10)
            .depends_on_being_done(&enter_ship_quest)
            .build();

        let learn_to_fly_quest = QuestBuilder::new(main_quest)
            .with_subquests([enter_ship_quest, move_ship_quest, rotate_ship_quest])
            .build();

        commands.entity(ent).insert(EnterShipQuestActive);
        ongoing_quests.start_quest(learn_to_fly_quest);
    }
}

#[derive(Component, Serialize, Deserialize, Default, Debug, Reflect)]
struct MoveShipQuestActive {
    distance_travelled: f32,
    braked: bool,
}

impl IdentifiableComponent for MoveShipQuestActive {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:rotate_ship_quest_active"
    }
}

impl DefaultPersistentComponent for MoveShipQuestActive {}

#[derive(Component, Serialize, Deserialize, Default, Debug, Reflect)]
struct RotateShipQuestActive {
    rotation_amount: f32,
}

impl IdentifiableComponent for RotateShipQuestActive {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:rotate_ship_quest_active"
    }
}

impl DefaultPersistentComponent for RotateShipQuestActive {}

#[derive(Component, Serialize, Deserialize)]
struct EnterShipQuestActive;

impl IdentifiableComponent for EnterShipQuestActive {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:enter_ship_quest_active"
    }
}

impl DefaultPersistentComponent for EnterShipQuestActive {}

fn resolve_enter_ship_quest(
    quests: Res<Registry<Quest>>,
    mut commands: Commands,
    mut q_on_quest_and_ready: Query<(Entity, &mut OngoingQuests), (With<Pilot>, With<EnterShipQuestActive>)>,
) {
    for (ent, mut ongoing_quests) in q_on_quest_and_ready.iter_mut() {
        let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        let Some(enter_ship_quest) = quests.from_id(ENTER_SHIP_QUEST) else {
            continue;
        };

        commands
            .entity(ent)
            .insert((MoveShipQuestActive::default(), RotateShipQuestActive::default()))
            .remove::<EnterShipQuestActive>();

        for ongoing in ongoing_quests.iter_specific_mut(quest) {
            if let Some(iterator) = ongoing
                .subquests_mut()
                .map(|subquests| subquests.iter_specific_mut(enter_ship_quest))
            {
                for ongoing in iterator {
                    ongoing.complete();
                }
            }
        }
    }
}

fn resolve_move_quest(
    quests: Res<Registry<Quest>>,
    mut commands: Commands,
    q_ship_vel: Query<(&Velocity, &ShipMovement), With<Ship>>,
    mut q_on_quest_and_ready: Query<(Entity, &mut OngoingQuests, &mut MoveShipQuestActive, &Pilot)>,
    time: Res<Time>,
) {
    for (ent, mut ongoing_quests, mut move_ship_quest_active, pilot) in q_on_quest_and_ready.iter_mut() {
        let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        let Some(move_ship_quest) = quests.from_id(MOVE_SHIP_QUEST) else {
            continue;
        };

        let Ok((ship_vel, ship_movement)) = q_ship_vel.get(pilot.entity) else {
            continue;
        };

        if ship_movement.braking {
            move_ship_quest_active.braked = true;
        }

        let distance_travelled = ship_vel.linvel.length() * time.delta_secs();

        move_ship_quest_active.distance_travelled += distance_travelled;

        for ongoing in ongoing_quests.iter_specific_mut(quest) {
            if let Some(iterator) = ongoing
                .subquests_mut()
                .map(|subquests| subquests.iter_specific_mut(move_ship_quest))
            {
                for ongoing in iterator {
                    let max_prog = ongoing.max_progress();
                    if ongoing.set_progress(
                        (max_prog - 1).min(move_ship_quest_active.distance_travelled.round() as u32) + move_ship_quest_active.braked as u32,
                    ) {
                        commands.entity(ent).remove::<MoveShipQuestActive>();
                    }
                }
            }
        }
    }
}

fn resolve_rotation_quest(
    quests: Res<Registry<Quest>>,
    mut commands: Commands,
    q_ship_vel: Query<&Velocity, With<Ship>>,
    time: Res<Time>,
    mut q_quest: Query<(Entity, &mut OngoingQuests, &mut RotateShipQuestActive, &Pilot)>,
) {
    for (ent, mut ongoing_quests, mut rotate_ship_quest_active, pilot) in q_quest.iter_mut() {
        let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        let Some(rotate_ship_quest) = quests.from_id(ROTATE_SHIP_QUEST) else {
            continue;
        };

        let Ok(ship_vel) = q_ship_vel.get(pilot.entity) else {
            continue;
        };

        rotate_ship_quest_active.rotation_amount += ship_vel.angvel.length() * time.delta_secs();

        for ongoing in ongoing_quests.iter_specific_mut(quest) {
            if let Some(iterator) = ongoing
                .subquests_mut()
                .map(|subquests| subquests.iter_specific_mut(rotate_ship_quest))
            {
                for ongoing in iterator {
                    if ongoing.set_progress(rotate_ship_quest_active.rotation_amount.round() as u32) {
                        commands.entity(ent).remove::<RotateShipQuestActive>();
                    }
                }
            }
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
    make_persistent::<EnterShipQuestActive>(app);
    make_persistent::<MoveShipQuestActive>(app);
    make_persistent::<RotateShipQuestActive>(app);

    app.add_systems(OnEnter(GameState::Loading), register_quest)
        .add_systems(
            FixedUpdate,
            (
                on_change_tutorial_state.in_set(QuestsSet::CreateNewQuests),
                (resolve_enter_ship_quest, resolve_move_quest, resolve_rotation_quest)
                    .after(on_change_tutorial_state)
                    .before(QuestsSet::CompleteQuests),
                on_complete_quest.after(QuestsSet::CompleteQuests),
            ),
        )
        .register_type::<MoveShipQuestActive>()
        .register_type::<RotateShipQuestActive>();
}
