use bevy::prelude::*;
use cosmos_core::{
    block::{Block, block_events::BlockBreakEvent},
    netty::sync::IdentifiableComponent,
    physics::location::Location,
    prelude::Structure,
    quest::{ActiveQuest, OngoingQuests, Quest, QuestBuilder},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::ship::pilot::{Pilot, PilotFocused},
    utils::quat_math::random_quat,
};
use rand::rng;
use serde::{Deserialize, Serialize};

use crate::{
    blocks::interactable::storage::OpenStorageEvent,
    loot::{LootTable, NeedsLootGenerated},
    persistence::{
        loading::NeedsBlueprintLoaded,
        make_persistent::{DefaultPersistentComponent, make_persistent},
    },
    quest::QuestsSet,
};

use super::TutorialState;

const MAIN_QUEST_NAME: &str = "cosmos:tutorial_collect_stash";
const FOCUS_STRUCTURE_QUEST: &str = "cosmos:tutorial_focus_structure";
const FLY_TO_STASH: &str = "cosmos:tutorial_fly_to_stash";
const COLLECT_ITEMS_QUEST: &str = "cosmos:tutorial_collect_items";

fn register_quest(mut quests: ResMut<Registry<Quest>>) {
    quests.register(Quest::new(MAIN_QUEST_NAME.to_string(), "Collect an abandon stash.".to_string()));
    quests.register(Quest::new(
        FOCUS_STRUCTURE_QUEST.to_string(),
        "Use <F> to 'focus' a waypoint while looking at it. The stash's waypoint is light blue.".to_string(),
    ));
    quests.register(Quest::new(FLY_TO_STASH.to_string(), "Fly to the abandon stash.".to_string()));
    quests.register(Quest::new(
        COLLECT_ITEMS_QUEST.to_string(),
        "Exit your ship using <F>. You can de-align yourself with your ship by pressing <L> - you can now rotate freely. Locate the storage container and take the items from it.".to_string(),
    ));
}

fn on_change_tutorial_state(
    mut q_quests: Query<
        (Entity, &mut OngoingQuests, &TutorialState, &Location),
        Or<(Changed<TutorialState>, (Added<OngoingQuests>, With<TutorialState>))>,
    >,
    quests: Res<Registry<Quest>>,
    mut commands: Commands,
    loot: Res<Registry<LootTable>>,
) {
    for (ent, mut ongoing_quests, tutorial_state, loc) in q_quests.iter_mut() {
        if *tutorial_state != TutorialState::CollectStash {
            continue;
        }

        let Some(main_quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        if ongoing_quests.contains(main_quest) {
            continue;
        }

        let Some(focus) = quests.from_id(FOCUS_STRUCTURE_QUEST) else {
            continue;
        };
        let Some(fly_to_stash) = quests.from_id(FLY_TO_STASH) else {
            continue;
        };
        let Some(collect_items) = quests.from_id(COLLECT_ITEMS_QUEST) else {
            continue;
        };

        let focus_quest = QuestBuilder::new(focus).build();
        let fly_to_stash_quest = QuestBuilder::new(fly_to_stash).depends_on_being_done(&focus_quest).build();
        let collect_items_quest = QuestBuilder::new(collect_items).depends_on_being_done(&fly_to_stash_quest).build();

        let main_quest = QuestBuilder::new(main_quest)
            .with_subquests([focus_quest, fly_to_stash_quest, collect_items_quest])
            .build();

        const STASH_DISTANCE: f32 = 2_000.0;

        commands.spawn((
            NeedsBlueprintLoaded {
                path: "default_blueprints/quests/tutorial/abandon_stash.bp".into(),
                spawn_at: *loc + random_quat(&mut rng()) * Vec3::new(0.0, 0.0, STASH_DISTANCE),
                rotation: random_quat(&mut rng()),
            },
            NeedsLootGenerated::from_loot_id("cosmos:tutorial_stash", &loot).expect("Missing tutorial_stash.json"),
            AbandonStash,
        ));

        let q_id = ongoing_quests.start_quest(main_quest);
        commands.entity(ent).insert(ActiveQuest(q_id));
    }
}

#[derive(Component, Serialize, Deserialize)]
struct AbandonStash;

impl IdentifiableComponent for AbandonStash {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:abandon_stash"
    }
}

impl DefaultPersistentComponent for AbandonStash {}

fn resolve_focus_waypoint_quest(
    quests: Res<Registry<Quest>>,
    mut q_on_quest_and_ready: Query<(&mut OngoingQuests, &Pilot)>,
    q_focused: Query<&PilotFocused>,
    q_abandon_stash: Query<(), With<AbandonStash>>,
) {
    for (mut ongoing_quests, pilot) in q_on_quest_and_ready.iter_mut() {
        let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        if !ongoing_quests.contains(quest) {
            continue;
        }

        let Some(enter_ship_quest) = quests.from_id(FOCUS_STRUCTURE_QUEST) else {
            continue;
        };

        if q_focused.get(pilot.entity).map(|x| !q_abandon_stash.contains(x.0)).unwrap_or(true) {
            continue;
        }

        for ongoing in ongoing_quests.iter_specific_mut(quest) {
            if let Some(iterator) = ongoing
                .subquests_mut()
                .map(|subquests| subquests.iter_specific_mut(enter_ship_quest).filter(|x| !x.completed()))
            {
                for ongoing in iterator {
                    ongoing.complete();
                }
            }
        }
    }
}

fn resolve_fly_ship_quest(
    quests: Res<Registry<Quest>>,
    mut q_ongoing_quest: Query<(&Location, &mut OngoingQuests)>,
    q_abandon_stash: Query<&Location, With<AbandonStash>>,
) {
    for (loc, mut ongoing_quests) in q_ongoing_quest.iter_mut() {
        let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        if !ongoing_quests.contains(quest) {
            continue;
        }

        let Some(fly_quest) = quests.from_id(FLY_TO_STASH) else {
            continue;
        };

        if !q_abandon_stash.iter().any(|x| x.distance_sqrd(loc) < 100.0 * 100.0) {
            continue;
        }

        for ongoing in ongoing_quests.iter_specific_mut(quest) {
            if let Some(iterator) = ongoing
                .subquests_mut()
                .map(|subquests| subquests.iter_specific_mut(fly_quest).filter(|x| !x.completed()))
            {
                for ongoing in iterator {
                    ongoing.complete();
                }
            }
        }
    }
}

fn resolve_loot_stash_quest(
    quests: Res<Registry<Quest>>,
    mut q_ongoing_quest: Query<&mut OngoingQuests>,
    mut evr_open_storage: EventReader<OpenStorageEvent>,
    mut evr_block_changed: EventReader<BlockBreakEvent>,
    q_abandon_stash: Query<&Structure, With<AbandonStash>>,
    blocks: Res<Registry<Block>>,
) {
    for ev in evr_block_changed.read() {
        let Ok(structure) = q_abandon_stash.get(ev.block.structure()) else {
            continue;
        };

        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:storage" {
            continue;
        }

        let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        let Ok(mut ongoing_quests) = q_ongoing_quest.get_mut(ev.breaker) else {
            continue;
        };

        if !ongoing_quests.contains(quest) {
            continue;
        }

        let Some(collect_items_quest) = quests.from_id(COLLECT_ITEMS_QUEST) else {
            continue;
        };

        for ongoing in ongoing_quests.iter_specific_mut(quest) {
            if let Some(iterator) = ongoing
                .subquests_mut()
                .map(|subquests| subquests.iter_specific_mut(collect_items_quest).filter(|x| !x.completed()))
            {
                for ongoing in iterator {
                    ongoing.complete();
                }
            }
        }
    }
    for ev in evr_open_storage.read() {
        let Ok(mut ongoing_quests) = q_ongoing_quest.get_mut(ev.player_ent) else {
            continue;
        };

        if !q_abandon_stash.contains(ev.block.structure()) {
            continue;
        }

        let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        if !ongoing_quests.contains(quest) {
            continue;
        }

        let Some(collect_items_quest) = quests.from_id(COLLECT_ITEMS_QUEST) else {
            continue;
        };

        for ongoing in ongoing_quests.iter_specific_mut(quest) {
            if let Some(iterator) = ongoing
                .subquests_mut()
                .map(|subquests| subquests.iter_specific_mut(collect_items_quest).filter(|x| !x.completed()))
            {
                for ongoing in iterator {
                    ongoing.complete();
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<AbandonStash>(app);

    super::add_tutorial(app, MAIN_QUEST_NAME);

    app.add_systems(OnEnter(GameState::PostLoading), register_quest).add_systems(
        FixedUpdate,
        (
            on_change_tutorial_state.in_set(QuestsSet::CreateNewQuests),
            (resolve_focus_waypoint_quest, resolve_fly_ship_quest, resolve_loot_stash_quest)
                .after(on_change_tutorial_state)
                .before(QuestsSet::CompleteQuests),
        ),
    );
}
