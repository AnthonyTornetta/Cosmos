use bevy::prelude::*;
use cosmos_core::{
    economy::Credits,
    ecs::sets::FixedUpdateSet,
    netty::sync::IdentifiableComponent,
    physics::location::{Location, SECTOR_DIMENSIONS},
    quest::{OngoingQuest, OngoingQuestDetails, OngoingQuestId, OngoingQuests, Quest},
    registry::Registry,
    state::GameState,
    structure::shared::MeltingDown,
    utils::random::random_range,
};
use serde::{Deserialize, Serialize};

use crate::{
    ai::hit_tracking::Hitters,
    persistence::make_persistent::{DefaultPersistentComponent, make_persistent},
    quest::AddQuestMessage,
    universe::spawners::pirate::{PirateNeedsSpawned, PirateSpawningSet},
};

pub const FIGHT_PIRATE_QUEST_NAME: &str = "cosmos:fight_pirate";

fn register_quest(mut quests: ResMut<Registry<Quest>>) {
    quests.register(Quest::new(FIGHT_PIRATE_QUEST_NAME.to_string(), "Fight a pirate".to_string()));
}

#[derive(Component, Debug, Serialize, Deserialize)]
struct FightPirateQuestNPC {
    quest_id: OngoingQuestId,
}

impl IdentifiableComponent for FightPirateQuestNPC {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:fight_pirate_quest_npc"
    }
}

impl DefaultPersistentComponent for FightPirateQuestNPC {}

fn on_add_quest(
    mut evr_add_quest: MessageReader<AddQuestMessage>,
    mut q_quests: Query<(&mut OngoingQuests, &Location)>,
    quests: Res<Registry<Quest>>,
    mut commands: Commands,
) {
    for ev in evr_add_quest.read() {
        if ev.unlocalized_name != FIGHT_PIRATE_QUEST_NAME {
            continue;
        }

        let Some(quest_entry) = quests.from_id(FIGHT_PIRATE_QUEST_NAME) else {
            continue;
        };

        let Ok((mut quests, loc)) = q_quests.get_mut(ev.to) else {
            continue;
        };

        let offset = Vec3::new(
            random_range(2.0 * SECTOR_DIMENSIONS, 3.0 * SECTOR_DIMENSIONS) * (rand::random::<f32>() - 0.5).signum(),
            random_range(2.0 * SECTOR_DIMENSIONS, 3.0 * SECTOR_DIMENSIONS) * (rand::random::<f32>() - 0.5).signum(),
            random_range(2.0 * SECTOR_DIMENSIONS, 3.0 * SECTOR_DIMENSIONS) * (rand::random::<f32>() - 0.5).signum(),
        );
        let location = *loc + offset;
        let details = OngoingQuestDetails {
            location: Some(location),
            ..ev.details.clone()
        };

        let quest_id = quests.start_quest(OngoingQuest::new(quest_entry, details, 3));

        let pirates = [2, 2, 3];

        for (i, &difficulty) in pirates.iter().enumerate() {
            commands.spawn((
                FightPirateQuestNPC { quest_id },
                PirateNeedsSpawned {
                    location: location + Vec3::new(0.0, i as f32 * 600.0, i as f32 * 700.0),
                    difficulty,
                    heading_towards: *loc,
                },
            ));
        }
    }
}

fn on_kill_pirates(
    mut commands: Commands,
    mut q_ongoing_quests: Query<(Entity, &mut OngoingQuests, &mut Credits)>,
    q_melting_down: Query<(Entity, &FightPirateQuestNPC, &Hitters), With<MeltingDown>>,
    q_not_melting_down: Query<&FightPirateQuestNPC, Without<MeltingDown>>,
) {
    for (entity, quest_npc, hitters) in q_melting_down.iter() {
        commands.entity(entity).remove::<FightPirateQuestNPC>();

        if q_not_melting_down.iter().any(|npc| quest_npc.quest_id == npc.quest_id) {
            // Quest is not yet complete
            continue;
        }

        for (ongoing_ent, mut ongoing, mut credits) in q_ongoing_quests.iter_mut() {
            let Some(ongoing_quest) = ongoing.remove_ongoing_quest(&quest_npc.quest_id) else {
                continue;
            };

            if hitters.get_number_of_hits(ongoing_ent) == 0 {
                continue;
            }

            if let Some(money) = ongoing_quest.details.payout {
                credits.increase(money.get() as u64);
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<FightPirateQuestNPC>(app);

    app.add_systems(OnEnter(GameState::Loading), register_quest).add_systems(
        FixedUpdate,
        (on_add_quest, on_kill_pirates)
            .chain()
            .before(PirateSpawningSet::PirateSpawningLogic)
            .in_set(FixedUpdateSet::Main),
    );
}
