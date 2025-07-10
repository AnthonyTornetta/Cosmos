use bevy::prelude::*;
use cosmos_core::{
    item::Item,
    physics::location::{Location, SECTOR_DIMENSIONS},
    quest::{OngoingQuests, Quest, QuestBuilder},
    registry::Registry,
    state::GameState,
};

use crate::{
    ai::hit_tracking::PlayerDestroyedNpcShipEvent,
    quest::{QuestsSet, quests::tutorial::add_tutorial},
    universe::{
        SectorDanger, UniverseSystems,
        spawners::pirate::{NextPirateSpawn, Pirate},
    },
};

use super::TutorialState;

const MAIN_QUEST_NAME: &str = "cosmos:tutorial_fight";
const FLY_TO_DANGER_ZONE_QUEST: &str = "cosmos:tutorial_fight_fly";
const WIN_FIGHT_QUEST: &str = "cosmos:tutorial_fight_win";
const MINE_SHIP_QUEST: &str = "cosmos:tutorial_mine_ship";

fn register_quest(mut quests: ResMut<Registry<Quest>>, items: Res<Registry<Item>>) {
    quests.register(Quest::new(MAIN_QUEST_NAME.to_string(), "Collect an abandon stash.".to_string()));
    quests.register(Quest::new(
        FLY_TO_DANGER_ZONE_QUEST.to_string(),
        "Once you feel ready for combat, fly to a dangerous area. Use your map (M) to find a dangerous region.".to_string(),
    ));
    quests.register(Quest::new(WIN_FIGHT_QUEST.to_string(), "Take out the pirate!".to_string()));
    quests.register(Quest::new_with_icon(
        MINE_SHIP_QUEST.to_string(),
        "Now that you destroyed that ship, you can use your plasma drills to mine it!".to_string(),
        items.from_id("cosmos:plasma_drill").expect("Missing plasma drill"),
    ));
}

fn on_change_tutorial_state(
    mut q_quests: Query<
        (&mut OngoingQuests, &TutorialState, &mut NextPirateSpawn),
        Or<(Changed<TutorialState>, (Added<OngoingQuests>, With<TutorialState>))>,
    >,
    quests: Res<Registry<Quest>>,
) {
    for (mut ongoing_quests, tutorial_state, mut next_pirate_spawn) in q_quests.iter_mut() {
        if *tutorial_state != TutorialState::Fight {
            continue;
        }

        let Some(main_quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        if ongoing_quests.contains(main_quest) {
            continue;
        }

        let Some(danger_area) = quests.from_id(FLY_TO_DANGER_ZONE_QUEST) else {
            continue;
        };
        let Some(win_fight) = quests.from_id(WIN_FIGHT_QUEST) else {
            continue;
        };

        let danger_area = QuestBuilder::new(danger_area).build();
        let win_fight = QuestBuilder::new(win_fight).depends_on_being_done(&danger_area).build();

        let main_quest = QuestBuilder::new(main_quest).with_subquests([danger_area, win_fight]).build();

        ongoing_quests.start_quest(main_quest);

        // This will spawn a pirate as soon as they enter a danger zone
        next_pirate_spawn.spawn_now();
    }
}

fn resolve_fly_to_dangerous_area(
    quests: Res<Registry<Quest>>,
    mut q_ongoing_quest: Query<(&Location, &mut OngoingQuests)>,
    q_pirates: Query<&Location, With<Pirate>>,
    universe: Res<UniverseSystems>,
) {
    for (loc, mut ongoing_quests) in q_ongoing_quest.iter_mut() {
        let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        let Some(fly_to_danger_zone) = quests.from_id(FLY_TO_DANGER_ZONE_QUEST) else {
            continue;
        };

        for ongoing in ongoing_quests.iter_specific_mut(quest) {
            if let Some(iterator) = ongoing
                .subquests_mut()
                .map(|subquests| subquests.iter_specific_mut(fly_to_danger_zone).filter(|x| !x.completed()))
            {
                if universe
                    .system(loc.get_system_coordinates())
                    .map(|x| x.sector_danger(loc.relative_sector()))
                    .unwrap_or_default()
                    > SectorDanger::MIDDLE
                    || q_pirates
                        .iter()
                        .any(|l| l.is_within_reasonable_range(loc) && l.distance_sqrd(loc) < SECTOR_DIMENSIONS * SECTOR_DIMENSIONS)
                {
                    for ongoing in iterator {
                        ongoing.complete();
                    }
                }
            }
        }
    }
}

fn resolve_kill_npc_ship(
    mut evr_player_killed_npc_ship: EventReader<PlayerDestroyedNpcShipEvent>,
    quests: Res<Registry<Quest>>,
    mut q_ongoing_quest: Query<&mut OngoingQuests>,
) {
    for ev in evr_player_killed_npc_ship.read() {
        let Ok(mut ongoing_quests) = q_ongoing_quest.get_mut(ev.player) else {
            continue;
        };
        let Some(quest) = quests.from_id(MAIN_QUEST_NAME) else {
            continue;
        };

        let Some(win_fight) = quests.from_id(WIN_FIGHT_QUEST) else {
            continue;
        };

        for ongoing in ongoing_quests.iter_specific_mut(quest) {
            if let Some(iterator) = ongoing
                .subquests_mut()
                .map(|subquests| subquests.iter_specific_mut(win_fight).filter(|x| !x.completed()))
            {
                for ongoing in iterator {
                    ongoing.complete();
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    add_tutorial(app, MAIN_QUEST_NAME);
    app.add_systems(OnEnter(GameState::PostLoading), register_quest).add_systems(
        FixedUpdate,
        (
            on_change_tutorial_state.in_set(QuestsSet::CreateNewQuests),
            (resolve_fly_to_dangerous_area, resolve_kill_npc_ship)
                .after(on_change_tutorial_state)
                .before(QuestsSet::CompleteQuests),
        ),
    );
}
