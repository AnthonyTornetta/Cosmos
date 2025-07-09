use bevy::prelude::*;
use cosmos_core::{
    item::Item,
    quest::{CompleteQuestEvent, OngoingQuests, Quest, QuestBuilder},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

use crate::{crafting::blocks::basic_fabricator::BasicFabricatorCraftEvent, quest::QuestsSet};

use super::TutorialState;

const MAIN_QUEST_NAME: &str = "cosmos:tutorial_craft";
const CRAFT_LASER_CANNON: &str = "cosmos:tutorial_craft_laser_cannon";
const CRAFT_PASSIVE_GEN: &str = "cosmos:tutorial_craft_passive_gen";
const CRAFT_PLASMA_DRILLS: &str = "cosmos:tutorial_craft_plasma_drills";

fn register_quest(mut quests: ResMut<Registry<Quest>>, items: Res<Registry<Item>>) {
    quests.register(Quest::new(
        MAIN_QUEST_NAME.to_string(),
        "Mine an asteroid - use the plasma drills".to_string(),
    ));

    if let Some(icon) = items.from_id("cosmos:plasma_drill") {
        quests.register(Quest::new_with_icon(
            CRAFT_PLASMA_DRILLS.to_string(),
            "Craft plasma drills".to_string(),
            icon,
        ));
    }
    if let Some(icon) = items.from_id("cosmos:laser_cannon") {
        quests.register(Quest::new_with_icon(
            CRAFT_LASER_CANNON.to_string(),
            "Craft passive generators".to_string(),
            icon,
        ));
    }
    if let Some(icon) = items.from_id("cosmos:passive_generator") {
        quests.register(Quest::new_with_icon(
            CRAFT_PASSIVE_GEN.to_string(),
            "Craft passive generators".to_string(),
            icon,
        ));
    }
}

fn on_change_tutorial_state(
    mut q_quests: Query<(&mut OngoingQuests, &TutorialState), Or<(Changed<TutorialState>, (Added<OngoingQuests>, With<TutorialState>))>>,
    quests: Res<Registry<Quest>>,
) {
    for (mut ongoing_quests, tutorial_state) in q_quests.iter_mut() {
        if *tutorial_state != TutorialState::Craft {
            continue;
        }

        let Some(main_quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        if ongoing_quests.contains(main_quest) {
            continue;
        }

        let Some(plasma_drills) = quests.from_id(CRAFT_PLASMA_DRILLS) else {
            continue;
        };
        let Some(passive_gen) = quests.from_id(CRAFT_PASSIVE_GEN) else {
            continue;
        };
        let Some(laser_cannon) = quests.from_id(CRAFT_LASER_CANNON) else {
            continue;
        };

        let plasma_drills = QuestBuilder::new(plasma_drills).with_max_progress(100).build();
        let passive_gen = QuestBuilder::new(passive_gen).with_max_progress(20).build();
        let laser_cannon = QuestBuilder::new(laser_cannon).with_max_progress(20).build();

        let main_quest = QuestBuilder::new(main_quest)
            .with_subquests([plasma_drills, passive_gen, laser_cannon])
            .build();

        ongoing_quests.start_quest(main_quest);
    }
}

fn resolve_quests(
    quests: Res<Registry<Quest>>,
    mut q_ongoing_quests: Query<&mut OngoingQuests>,
    items: Res<Registry<Item>>,
    mut evr_craft: EventReader<BasicFabricatorCraftEvent>,
) {
    let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
        return;
    };

    for ev in evr_craft.read() {
        let Ok(mut ongoing_quests) = q_ongoing_quests.get_mut(ev.crafter) else {
            continue;
        };

        let Some(ongoing) = ongoing_quests.get_quest_mut(quest) else {
            continue;
        };

        let Some(subquests) = ongoing.subquests_mut() else {
            continue;
        };

        match items.from_numeric_id(ev.item_crafted).unlocalized_name() {
            "cosmos:passive_generator" => {
                let Some(quest) = quests.from_id(CRAFT_PASSIVE_GEN) else {
                    continue;
                };
                if let Some(quest) = subquests.get_quest_mut(quest) {
                    quest.progress_quest(ev.quantity);
                }
            }
            "cosmos:laser_cannon" => {
                let Some(quest) = quests.from_id(CRAFT_LASER_CANNON) else {
                    continue;
                };
                if let Some(quest) = subquests.get_quest_mut(quest) {
                    quest.progress_quest(ev.quantity);
                }
            }
            "cosmos:plasma_drill" => {
                let Some(quest) = quests.from_id(CRAFT_PLASMA_DRILLS) else {
                    continue;
                };
                if let Some(quest) = subquests.get_quest_mut(quest) {
                    quest.progress_quest(ev.quantity);
                }
            }
            _ => {}
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
