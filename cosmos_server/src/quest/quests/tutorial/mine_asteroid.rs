use bevy::prelude::*;
use cosmos_core::{
    block::{Block, block_events::BlockBreakMessage},
    item::Item,
    quest::{ActiveQuest, OngoingQuests, Quest, QuestBuilder},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::ship::pilot::Pilot,
};

use crate::quest::QuestsSet;

use super::TutorialState;

const MAIN_QUEST_NAME: &str = "cosmos:tutorial_mine_asteroid";
const MINE_IRON: &str = "cosmos:tutorial_mine_iron";
const MINE_COPPER: &str = "cosmos:tutorial_mine_copper";
const MINE_ENERGITE: &str = "cosmos:tutorial_mine_energite";
const MINE_PHOTONIUM: &str = "cosmos:tutorial_mine_photonium";

fn register_quest(mut quests: ResMut<Registry<Quest>>, items: Res<Registry<Item>>) {
    quests.register(Quest::new(
        MAIN_QUEST_NAME.to_string(),
        "Mine an asteroid - use the plasma drills".to_string(),
    ));

    if let Some(iron_ore) = items.from_id("cosmos:iron_ore") {
        quests.register(Quest::new_with_icon(
            MINE_IRON.to_string(),
            "Iron the provides strength and structure for most ship components and hull.".to_string(),
            iron_ore,
        ));
    }
    if let Some(copper_ore) = items.from_id("cosmos:copper_ore") {
        quests.register(Quest::new_with_icon(
            MINE_COPPER.to_string(),
            "Copper's highly conductive nature makes it essential for any electronic devices.".to_string(),
            copper_ore,
        ));
    }
    if let Some(energite_ore) = items.from_id("cosmos:energite_crystal_ore") {
        quests.register(Quest::new_with_icon(
            MINE_ENERGITE.to_string(),
            "Energite's properties make it ideal for anything that creates or stores power.".to_string(),
            energite_ore,
        ));
    }
    if let Some(photonium_ore) = items.from_id("cosmos:photonium_crystal_ore") {
        quests.register(Quest::new_with_icon(
            MINE_PHOTONIUM.to_string(),
            "Photonium focuses light into high-density energy packets we call lasers.".to_string(),
            photonium_ore,
        ));
    }
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
        if *tutorial_state != TutorialState::MineAsteroid {
            continue;
        }

        let Some(main_quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        if ongoing_quests.contains(main_quest) {
            continue;
        }

        let Some(copper) = quests.from_id(MINE_COPPER) else {
            continue;
        };
        let Some(iron) = quests.from_id(MINE_IRON) else {
            continue;
        };
        let Some(energite) = quests.from_id(MINE_ENERGITE) else {
            continue;
        };
        let Some(photonium) = quests.from_id(MINE_PHOTONIUM) else {
            continue;
        };

        let copper = QuestBuilder::new(copper).with_max_progress(100).build();
        let iron = QuestBuilder::new(iron).with_max_progress(100).build();
        let energite = QuestBuilder::new(energite).with_max_progress(20).build();
        let photonium = QuestBuilder::new(photonium).with_max_progress(30).build();

        let main_quest = QuestBuilder::new(main_quest)
            .with_subquests([copper, iron, energite, photonium])
            .build();

        let q_id = ongoing_quests.start_quest(main_quest);
        commands.entity(ent).insert(ActiveQuest(q_id));
    }
}

fn resolve_quests(
    quests: Res<Registry<Quest>>,
    mut q_ongoing_quests: Query<&mut OngoingQuests>,
    mut evr_block_break: MessageReader<BlockBreakMessage>,
    q_pilot: Query<&Pilot>,
    blocks: Res<Registry<Block>>,
) {
    let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
        return;
    };

    for ev in evr_block_break.read() {
        let mut ongoing_quests = q_ongoing_quests.get_mut(ev.breaker);

        if ongoing_quests.is_err() {
            let Ok(pilot) = q_pilot.get(ev.breaker) else {
                continue;
            };
            ongoing_quests = q_ongoing_quests.get_mut(pilot.entity);
        }

        let Ok(mut ongoing_quests) = ongoing_quests else {
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

        match blocks.from_numeric_id(ev.broken_id).unlocalized_name() {
            "cosmos:copper_ore" => {
                let Some(quest) = quests.from_id(MINE_COPPER) else {
                    continue;
                };
                if let Some(quest) = subquests.get_quest_mut(quest) {
                    quest.progress_quest(1);
                }
            }
            "cosmos:iron_ore" => {
                let Some(quest) = quests.from_id(MINE_IRON) else {
                    continue;
                };
                if let Some(quest) = subquests.get_quest_mut(quest) {
                    quest.progress_quest(1);
                }
            }
            "cosmos:energite_crystal_ore" => {
                let Some(quest) = quests.from_id(MINE_ENERGITE) else {
                    continue;
                };
                if let Some(quest) = subquests.get_quest_mut(quest) {
                    quest.progress_quest(1);
                }
            }
            "cosmos:photonium_crystal_ore" => {
                let Some(quest) = quests.from_id(MINE_PHOTONIUM) else {
                    continue;
                };
                if let Some(quest) = subquests.get_quest_mut(quest) {
                    quest.progress_quest(1);
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
