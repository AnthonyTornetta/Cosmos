//! Keeps track of the entities that have hit other AI-Controlled ships, if the [`Hitters`]
//! component is added.

use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer, utils::HashMap};
use cosmos_core::{
    block::block_events::BlockEventsSet,
    entities::EntityId,
    faction::{FactionId, FactionRelation, Factions},
    netty::system_sets::NetworkingSystemsSet,
    state::GameState,
    structure::{block_health::events::BlockTakeDamageEvent, shared::MeltingDown, ship::pilot::Pilot},
};
use serde::{Deserialize, Serialize};

use crate::entities::player::strength::{PlayerStrength, PlayerStrengthSystemSet};

use super::AiControlled;

#[derive(Component, Default, Reflect, Debug)]
/// Keeps track of entities that have hit this ship
pub struct Hitters(HashMap<Entity, u64>);

impl Hitters {
    /// Gets the number of times this entity has hit this ship. This does not account for relative
    /// damage.
    pub fn get_number_of_hits(&self, ent: Entity) -> u64 {
        self.0.get(&ent).copied().unwrap_or(0)
    }
}

/// How much the difficulty will increase after killing this entity.
/// This is evenly divided between the players that killed this, based on how many times each
/// player hit it.
///
/// Sample values:
/// - Basic Pirate Ship = 5.0
#[derive(Component, Default, Reflect, Debug, Serialize, Deserialize, Clone, Copy)]
pub struct DifficultyIncreaseOnKill(pub f32);

fn process_hit_events(mut q_hitters: Query<&mut Hitters>, q_pilot: Query<&Pilot>, mut evr_hit_block: EventReader<BlockTakeDamageEvent>) {
    for ev in evr_hit_block.read() {
        let Some(causer) = ev.causer else {
            continue;
        };

        let causer = q_pilot.get(causer).map(|x| x.entity).unwrap_or(causer);

        let Ok(mut hitters) = q_hitters.get_mut(ev.structure_entity) else {
            continue;
        };

        *hitters.as_mut().0.entry(causer).or_default() += 1;
    }
}

const HITS_FOR_WAR: u64 = 10;

fn add_faction_enemies(
    mut commands: Commands,
    mut factions: ResMut<Factions>,
    q_fac_id: Query<&FactionId>,
    q_entity_id: Query<&EntityId>,
    q_hitter: Query<(&Hitters, &FactionId), Changed<Hitters>>,
) {
    for (hitters, faction_id) in q_hitter.iter() {
        for (&ent, &amt) in hitters.0.iter() {
            if amt < HITS_FOR_WAR {
                continue;
            }

            let Some(fac) = factions.from_id(faction_id) else {
                continue;
            };

            let ent_id = q_entity_id.get(ent).ok().cloned();
            let ent_id = if let Some(eid) = ent_id {
                eid
            } else {
                let eid = EntityId::generate();
                commands.entity(ent).insert(eid.clone());
                eid
            };

            let hitter_fac_id = q_fac_id.get(ent).ok();

            let hitter_fac = hitter_fac_id.map(|x| factions.from_id(x)).flatten();
            let relation = fac.relation_with_entity(&ent_id, hitter_fac);

            if relation == FactionRelation::Neutral {
                factions.set_relation(faction_id, hitter_fac_id, Some(&ent_id), FactionRelation::Enemy);
            }
        }
    }
}

fn tick_down_hitters(mut q_hitters: Query<&mut Hitters>) {
    for mut hitter in q_hitters.iter_mut() {
        hitter.as_mut().0.retain(|_, count| {
            *count -= 1;
            *count > 0
        });
    }
}

fn add_hitters(mut commands: Commands, q_needs_hitter: Query<Entity, (With<AiControlled>, Without<Hitters>)>) {
    for ent in q_needs_hitter.iter() {
        commands.entity(ent).insert(Hitters::default());
    }
}

fn on_melt_down(
    mut q_players: Query<&mut PlayerStrength>,
    q_melting_down: Query<(&Hitters, &DifficultyIncreaseOnKill), Added<MeltingDown>>,
) {
    for (hitters, difficulty_increase) in q_melting_down.iter() {
        let dmg_total = hitters.0.iter().map(|(_, hits)| *hits).sum::<u64>();

        for (&hitter_ent, &hits) in hitters.0.iter() {
            let percent = hits as f32 / dmg_total as f32;
            let Ok(mut player_strength) = q_players.get_mut(hitter_ent) else {
                warn!("No player strength!");
                continue;
            };

            player_strength.0 += percent * difficulty_increase.0;
            player_strength.0 = player_strength.0.clamp(0.0, 100.0);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (add_hitters, process_hit_events, add_faction_enemies)
            .chain()
            .after(PlayerStrengthSystemSet::UpdatePlayerStrength)
            .after(tick_down_hitters)
            .run_if(in_state(GameState::Playing))
            .in_set(BlockEventsSet::ProcessEvents),
    )
    .add_systems(
        Update,
        on_melt_down
            .run_if(in_state(GameState::Playing))
            .after(process_hit_events)
            .in_set(NetworkingSystemsSet::Between),
    )
    .add_systems(
        Update,
        tick_down_hitters
            .run_if(in_state(GameState::Playing))
            .run_if(on_timer(Duration::from_secs(1))),
    )
    .register_type::<Hitters>()
    .register_type::<DifficultyIncreaseOnKill>();
}
