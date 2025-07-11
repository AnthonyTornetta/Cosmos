use bevy::prelude::*;
use cosmos_core::{
    item::Item,
    quest::{OngoingQuests, Quest, QuestBuilder},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

use crate::{crafting::blocks::basic_fabricator::BasicFabricatorCraftEvent, quest::QuestsSet};

use super::TutorialState;

const MAIN_QUEST_NAME: &str = "cosmos:tutorial_craft";
const CRAFT_MISSILE_LAUNCHERS: &str = "cosmos:tutorial_craft_missile_launchers";
const CRAFT_PASSIVE_GEN: &str = "cosmos:tutorial_craft_passive_gen";
const CRAFT_PLASMA_DRILLS: &str = "cosmos:tutorial_craft_plasma_drills";
const CRAFT_MISSILE: &str = "cosmos:tutorial_craft_missile";

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
    if let Some(icon) = items.from_id("cosmos:missile_launcher") {
        quests.register(Quest::new_with_icon(
            CRAFT_MISSILE_LAUNCHERS.to_string(),
            "Craft missile launchers".to_string(),
            icon,
        ));
    }
    if let Some(icon) = items.from_id("cosmos:missile") {
        quests.register(Quest::new_with_icon(CRAFT_MISSILE.to_string(), "Craft missiles".to_string(), icon));
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
        let Some(missile_launcher) = quests.from_id(CRAFT_MISSILE_LAUNCHERS) else {
            continue;
        };
        let Some(missile) = quests.from_id(CRAFT_MISSILE) else {
            continue;
        };

        let plasma_drills = QuestBuilder::new(plasma_drills).with_max_progress(100).build();
        let passive_gen = QuestBuilder::new(passive_gen).with_max_progress(20).build();
        let missile_launcher = QuestBuilder::new(missile_launcher).with_max_progress(20).build();
        let missile = QuestBuilder::new(missile).with_max_progress(20).build();

        let main_quest = QuestBuilder::new(main_quest)
            .with_subquests([plasma_drills, passive_gen, missile_launcher, missile])
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

        if !ongoing_quests.contains(quest) {
            continue;
        }

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
            "cosmos:missile_launcher" => {
                let Some(quest) = quests.from_id(CRAFT_MISSILE_LAUNCHERS) else {
                    continue;
                };
                if let Some(quest) = subquests.get_quest_mut(quest) {
                    quest.progress_quest(ev.quantity);
                }
            }
            "cosmos:missile" => {
                let Some(quest) = quests.from_id(CRAFT_MISSILE) else {
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
