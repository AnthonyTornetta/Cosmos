use bevy::prelude::*;
use cosmos_core::{
    block::{Block, block_events::BlockPlaceEvent, data::BlockData},
    ecs::mut_events::MutEvent,
    item::Item,
    prelude::Structure,
    quest::{CompleteQuestEvent, OngoingQuests, Quest, QuestBuilder},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

use crate::{inventory::InventoryAddItemEvent, quest::QuestsSet};

use super::TutorialState;

const MAIN_QUEST_NAME: &str = "cosmos:tutorial_arm_ship";
const PLACE_MISSILE_LAUNCHER_QUEST: &str = "cosmos:tutorial_arm_ship_missile_launcher";
const PLACE_STORAGE_QUEST: &str = "cosmos:tutorial_place_storage";
const INSERT_MISSILES_QUEST: &str = "cosmos:tutorial_insert_missiles";

const N_MISSILE_LAUNCHERS: u32 = 40;
const N_MISSILES: u32 = 40;

fn register_quest(mut quests: ResMut<Registry<Quest>>) {
    quests.register(Quest::new(
        MAIN_QUEST_NAME.to_string(),
        "Build a ship to mine asteroids and explore.".to_string(),
    ));
    quests.register(Quest::new(
        PLACE_MISSILE_LAUNCHER_QUEST.to_string(),
        format!("Place at least {N_MISSILE_LAUNCHERS} missile launchers. These can be fired at a target to deal damage to their ship. These deal more damage if placed in a line with each other.")
    ));
    quests.register(Quest::new(
        PLACE_STORAGE_QUEST.to_string(),
        format!("Place at least one storage unit on the ship."),
    ));
    quests.register(Quest::new(
        INSERT_MISSILES_QUEST.to_string(),
        format!("Insert at least {N_MISSILES} missiles into the storage you placed. Missile launchers use these as ammunition."),
    ));
}

fn on_change_tutorial_state(
    mut q_quests: Query<(&mut OngoingQuests, &TutorialState), Or<(Changed<TutorialState>, (Added<OngoingQuests>, With<TutorialState>))>>,
    quests: Res<Registry<Quest>>,
) {
    for (mut ongoing_quests, tutorial_state) in q_quests.iter_mut() {
        if *tutorial_state != TutorialState::ArmShip {
            continue;
        }

        let Some(main_quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        if ongoing_quests.contains(main_quest) {
            continue;
        }

        let Some(missile_launchers) = quests.from_id(PLACE_MISSILE_LAUNCHER_QUEST) else {
            continue;
        };
        let Some(storage) = quests.from_id(PLACE_STORAGE_QUEST) else {
            continue;
        };
        let Some(missiles) = quests.from_id(INSERT_MISSILES_QUEST) else {
            continue;
        };

        let missile_launchers = QuestBuilder::new(missile_launchers).with_max_progress(N_MISSILE_LAUNCHERS).build();
        let storage = QuestBuilder::new(storage).with_max_progress(1).build();
        let missiles = QuestBuilder::new(missiles).with_max_progress(N_MISSILES).build();

        let main_quest = QuestBuilder::new(main_quest)
            .with_subquests([missile_launchers, storage, missiles])
            .build();

        ongoing_quests.start_quest(main_quest);
    }
}

fn resolve_quests(
    quests: Res<Registry<Quest>>,
    mut q_on_quest_and_ready: Query<&mut OngoingQuests>,
    mut evr_block_placed: EventReader<MutEvent<BlockPlaceEvent>>,
    mut evr_inventory_added: EventReader<InventoryAddItemEvent>,
    blocks: Res<Registry<Block>>,
    items: Res<Registry<Item>>,
    q_structure: Query<&Structure>,
    q_block_data: Query<&BlockData>,
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
            "cosmos:missile_launcher" => {
                advance_subquest(&quests, &mut ongoing_quests, PLACE_MISSILE_LAUNCHER_QUEST, 1);
            }
            "cosmos:storage" => {
                advance_subquest(&quests, &mut ongoing_quests, PLACE_STORAGE_QUEST, 1);
            }
            _ => {}
        }
    }

    for ev in evr_inventory_added.read() {
        let Some(adder) = ev.adder else {
            continue;
        };
        let Ok(mut ongoing_quests) = q_on_quest_and_ready.get_mut(adder) else {
            continue;
        };

        match items.from_numeric_id(ev.item.item_id).unlocalized_name() {
            "cosmos:missile" => {
                if let Ok(bd) = q_block_data.get(ev.inventory_entity) {
                    if let Ok(structure) = q_structure.get(bd.identifier.block.structure()) {
                        if structure.block_at(bd.identifier.block.coords(), &blocks).unlocalized_name() == "cosmos:storage" {
                            advance_subquest(&quests, &mut ongoing_quests, INSERT_MISSILES_QUEST, ev.item.amount as u32);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn advance_subquest(quests: &Registry<Quest>, ongoing_quests: &mut Mut<'_, OngoingQuests>, quest_name: &str, amt: u32) {
    let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
        return;
    };

    let Some(place_quest) = quests.from_id(quest_name) else {
        return;
    };

    info!("{:?}", ongoing_quests.as_ref());

    for ongoing in ongoing_quests.iter_specific_mut(quest) {
        if let Some(iterator) = ongoing
            .subquests_mut()
            .map(|subquests| subquests.iter_specific_mut(place_quest).filter(|x| !x.completed()))
        {
            for ongoing in iterator {
                ongoing.progress_quest(amt);
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
    app.add_systems(OnEnter(GameState::PostLoading), register_quest).add_systems(
        FixedUpdate,
        (
            on_change_tutorial_state.in_set(QuestsSet::CreateNewQuests),
            resolve_quests.after(on_change_tutorial_state).before(QuestsSet::CompleteQuests),
            on_complete_quest.after(QuestsSet::CompleteQuests),
        ),
    );
}
