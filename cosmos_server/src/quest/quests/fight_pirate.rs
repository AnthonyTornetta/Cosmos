use bevy::prelude::*;
use cosmos_core::{
    netty::system_sets::NetworkingSystemsSet,
    physics::location::{Location, SECTOR_DIMENSIONS},
    quest::{OngoingQuestDetails, OngoingQuests, Quest},
    registry::Registry,
    state::GameState,
    utils::random::random_range,
};
use serde::{Deserialize, Serialize};

use crate::{
    quest::AddQuestEvent,
    universe::spawners::pirate::{PirateNeedsSpawned, PirateSpawningSet},
};

pub const FIGHT_PIRATE_QUEST_NAME: &str = "cosmos:fight_pirate";

fn register_quest(mut quests: ResMut<Registry<Quest>>) {
    quests.register(Quest::new(FIGHT_PIRATE_QUEST_NAME.to_string(), "Fight a pirate".to_string()));
}

#[derive(Component, Debug, Serialize, Deserialize)]
struct FightPirateQuestNPC {
    quest_holder: Entity,
}

fn on_add_quest(
    mut evr_add_quest: EventReader<AddQuestEvent>,
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

        quests.start_quest(quest_entry, details);

        commands.spawn((
            FightPirateQuestNPC { quest_holder: ev.to },
            PirateNeedsSpawned {
                location,
                difficulty: 2,
                heading_towards: *loc,
            },
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Loading), register_quest).add_systems(
        Update,
        on_add_quest
            .before(PirateSpawningSet::PirateSpawningLogic)
            .in_set(NetworkingSystemsSet::Between),
    );
}
