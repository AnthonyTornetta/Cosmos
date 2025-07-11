use bevy::prelude::*;
use cosmos_core::{
    block::{Block, block_events::BlockPlaceEvent},
    ecs::mut_events::MutEvent,
    quest::{OngoingQuests, Quest, QuestBuilder},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

use crate::quest::QuestsSet;

use super::TutorialState;

const MAIN_QUEST_NAME: &str = "cosmos:tutorial_build_ship";
const PLACE_THRUSTERS_QUEST: &str = "cosmos:tutorial_place_thrusters";
const PLACE_PLASMA_DRILL_QUEST: &str = "cosmos:tutorial_place_plasma_drill";
const PLACE_PASSIVE_GENERATOR_QUEST: &str = "cosmos:tutorial_place_passive_generator";
const PLACE_ENERGY_CELL_QUEST: &str = "cosmos:tutorial_place_energy_cell";
const PLACE_LASER_CANNON_QUEST: &str = "cosmos:tutorial_place_laser_cannon";

const N_THRUSTERS: u32 = 10;
const N_PLASMA_DRILLS: u32 = 30;
const N_PASSIVE_GENS: u32 = 30;
const N_ENERGY_CELLS: u32 = 1;
const N_LASER_CANNONS: u32 = 30;

fn register_quest(mut quests: ResMut<Registry<Quest>>) {
    quests.register(Quest::new(
        MAIN_QUEST_NAME.to_string(),
        "Build a ship to mine asteroids and explore.".to_string(),
    ));
    quests.register(Quest::new(
        PLACE_THRUSTERS_QUEST.to_string(),
        format!("Place {N_THRUSTERS} thrusters. These allow you to move faster and maneuver better."),
    ));
    quests.register(Quest::new(
        PLACE_PLASMA_DRILL_QUEST.to_string(),
        format!("Place {N_PLASMA_DRILLS} plasma drills. These allow you to mine asteroids and decaying ships. These drill much faster if placed in a line with each other."),
    ));
    quests.register(Quest::new(
        PLACE_PASSIVE_GENERATOR_QUEST.to_string(),
        format!("Place {N_PASSIVE_GENS} passive generators. These generate a small amount of energy passively."),
    ));
    quests.register(Quest::new(
        PLACE_ENERGY_CELL_QUEST.to_string(),
        "Place energy cells. These allow you to store large amounts of energy.".to_string(),
    ));
    quests.register(Quest::new(
        PLACE_LASER_CANNON_QUEST.to_string(),
        format!("Place {N_LASER_CANNONS} laser cannons. These can be fired at a target to deal damage to their ship. These deal more damage if placed in a line with each other.")
    ));
}

fn on_change_tutorial_state(
    mut q_quests: Query<(&mut OngoingQuests, &TutorialState), Or<(Changed<TutorialState>, (Added<OngoingQuests>, With<TutorialState>))>>,
    quests: Res<Registry<Quest>>,
) {
    for (mut ongoing_quests, tutorial_state) in q_quests.iter_mut() {
        if *tutorial_state != TutorialState::BuildShip {
            continue;
        }

        let Some(main_quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        if ongoing_quests.contains(main_quest) {
            continue;
        }

        let Some(thrusters) = quests.from_id(PLACE_THRUSTERS_QUEST) else {
            continue;
        };
        let Some(drills) = quests.from_id(PLACE_PLASMA_DRILL_QUEST) else {
            continue;
        };
        let Some(passive_gens) = quests.from_id(PLACE_PASSIVE_GENERATOR_QUEST) else {
            continue;
        };
        let Some(energy_cells) = quests.from_id(PLACE_ENERGY_CELL_QUEST) else {
            continue;
        };
        let Some(laser_cannons) = quests.from_id(PLACE_LASER_CANNON_QUEST) else {
            continue;
        };

        let thrusters = QuestBuilder::new(thrusters).with_max_progress(N_THRUSTERS).build();
        let drills = QuestBuilder::new(drills).with_max_progress(N_PLASMA_DRILLS).build();
        let passive_gens = QuestBuilder::new(passive_gens).with_max_progress(N_PASSIVE_GENS).build();
        let energy_cells = QuestBuilder::new(energy_cells).with_max_progress(N_ENERGY_CELLS).build();
        let laser_cannons = QuestBuilder::new(laser_cannons).with_max_progress(N_LASER_CANNONS).build();

        let main_quest = QuestBuilder::new(main_quest)
            .with_subquests([thrusters, drills, passive_gens, energy_cells, laser_cannons])
            .build();

        ongoing_quests.start_quest(main_quest);
    }
}

fn resolve_quests(
    quests: Res<Registry<Quest>>,
    mut q_on_quest_and_ready: Query<&mut OngoingQuests>,
    mut evr_block_placed: EventReader<MutEvent<BlockPlaceEvent>>,
    blocks: Res<Registry<Block>>,
) {
    for ev in evr_block_placed.read() {
        let ev = ev.read();
        let BlockPlaceEvent::Event(ev) = &*ev else {
            continue;
        };

        let Ok(mut ongoing_quests) = q_on_quest_and_ready.get_mut(ev.placer) else {
            continue;
        };

        match blocks.from_numeric_id(ev.block_id).unlocalized_name() {
            "cosmos:thruster" => {
                advance_place_quest(&quests, &mut ongoing_quests, PLACE_THRUSTERS_QUEST);
            }
            "cosmos:laser_cannon" => {
                advance_place_quest(&quests, &mut ongoing_quests, PLACE_LASER_CANNON_QUEST);
            }
            "cosmos:plasma_drill" => {
                advance_place_quest(&quests, &mut ongoing_quests, PLACE_PLASMA_DRILL_QUEST);
            }
            "cosmos:energy_cell" => {
                advance_place_quest(&quests, &mut ongoing_quests, PLACE_ENERGY_CELL_QUEST);
            }
            "cosmos:passive_generator" => {
                advance_place_quest(&quests, &mut ongoing_quests, PLACE_PASSIVE_GENERATOR_QUEST);
            }
            _ => {}
        }
    }
}

fn advance_place_quest(quests: &Registry<Quest>, ongoing_quests: &mut Mut<'_, OngoingQuests>, quest_name: &str) {
    let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
        return;
    };

    if !ongoing_quests.contains(quest) {
        return;
    }

    let Some(place_quest) = quests.from_id(quest_name) else {
        return;
    };

    for ongoing in ongoing_quests.iter_specific_mut(quest) {
        if let Some(iterator) = ongoing
            .subquests_mut()
            .map(|subquests| subquests.iter_specific_mut(place_quest).filter(|x| !x.completed()))
        {
            for ongoing in iterator {
                ongoing.progress_quest(1);
            }
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
