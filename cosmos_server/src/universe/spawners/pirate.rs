//! Spawns pirate ships

use std::time::Duration;

use bevy::{platform::collections::HashMap, prelude::*, time::common_conditions::on_timer};
use cosmos_core::{
    entities::player::Player,
    physics::location::{Location, SECTOR_DIMENSIONS, Sector, SectorUnit},
    state::GameState,
    utils::{quat_math::QuatMath, random::random_range},
};

use crate::{
    entities::player::strength::{PlayerStrength, TotalTimePlayed},
    persistence::loading::{LoadingBlueprintSystemSet, NeedsBlueprintLoaded},
    settings::ServerSettings,
    universe::{SectorDanger, UniverseSystems},
};

/// TODO: Load this from config
///
/// Total playtime (sec) is divided by this to calculate its difficulty.
const TIME_DIFFICULTY_CONSTANT: f32 = 80_000.0;

/// TODO: Load this from config
///
/// How much one player's strength will increase the pirate's difficulty.
const PLAYER_STRENGTH_INCREASE_FACTOR: f32 = 1.0;

#[derive(Component)]
/// A pirate needs spawned for this entity, please add the components it needs to function
pub struct PirateNeedsSpawned {
    /// The location this pirate should be spawned
    pub location: Location,
    /// The difficulty of this pirate (used to load the appropriate blueprint)
    pub difficulty: u32,
    /// Where the pirate should face and head towards
    pub heading_towards: Location,
}

#[derive(Component)]
/// A pirate-controlled ship
pub struct Pirate;

/// The maximum difficulty of ship we can spawn. This is NOT the total difficulty.
///
/// Difficulty range is [0, MAX_DIFFICULTY]
pub const MAX_PIRATE_DIFFICULTY: u64 = 3;

fn on_needs_pirate_spawned(mut commands: Commands, q_needs_pirate_spawned: Query<(Entity, &PirateNeedsSpawned)>) {
    for (ent, pns) in q_needs_pirate_spawned.iter() {
        let difficulty = pns.difficulty;

        let rotation = (pns.heading_towards - pns.location).absolute_coords_f32().normalize_or_zero();

        commands.entity(ent).remove::<PirateNeedsSpawned>().insert((
            Pirate,
            NeedsBlueprintLoaded {
                path: format!("default_blueprints/pirate/default_{difficulty}.bp"),
                rotation: Quat::looking_to(rotation, Vec3::Y),
                spawn_at: pns.location,
            },
        ));
    }
}

#[derive(Component, Clone, Copy, PartialEq, PartialOrd, Reflect, Debug)]
/// Goes on the player and ensures they don't deal with too many pirates
struct NextPirateSpawn(f64);

fn add_spawn_times(
    q_players: Query<Entity, (With<Player>, Without<NextPirateSpawn>)>,
    time: Res<Time>,
    min_pirate_spawn_time: Res<MinPirateSpawnTime>,
    mut commands: Commands,
) {
    for ent in q_players.iter() {
        let next_spawn_time = calculate_next_spawn_time(&time, &min_pirate_spawn_time);
        commands.entity(ent).insert(NextPirateSpawn(next_spawn_time));
    }
}

fn spawn_pirates(
    mut commands: Commands,
    mut q_players: Query<(Entity, &Location, &mut NextPirateSpawn, &TotalTimePlayed, &PlayerStrength), With<Player>>,
    time: Res<Time>,
    min_pirate_spawn_time: Res<MinPirateSpawnTime>,
    server_settings: Res<ServerSettings>,
    universe: Res<UniverseSystems>,
) {
    if server_settings.peaceful {
        return;
    }

    let mut player_groups: HashMap<Sector, (NextPirateSpawn, Vec<Entity>, TotalTimePlayed, PlayerStrength)> = HashMap::default();

    const MAX_DIST: f32 = SECTOR_DIMENSIONS * 2.0 + 20.0;

    for (player_ent, player_loc, mut player_next_pirate_spawn, total_time_played, player_strength) in q_players.iter_mut() {
        let danger = universe
            .system(player_loc.get_system_coordinates())
            .map(|x| x.sector_danger(player_loc.relative_sector()))
            .unwrap_or_default();

        if danger <= SectorDanger::MIDDLE {
            // 1.0 means the pirate spawn time is pushed back forever until leaving, any lower will
            // still push the time back, just not the full amount
            const PIRATE_SPAWN_DELAY_AMOUNT: f64 = 0.75;
            player_next_pirate_spawn.0 += time.delta_secs_f64() * PIRATE_SPAWN_DELAY_AMOUNT;
            continue;
        }

        if let Some(sec) = player_groups
            .keys()
            .find(|&sec| {
                player_loc.is_within_reasonable_range_sector(*sec)
                    && Location::new(Vec3::ZERO, *sec - player_loc.sector).distance_sqrd(&Location::ZERO) <= MAX_DIST * MAX_DIST
            })
            .copied()
        {
            let (last_pirate_spawn, ents, cur_total_time, cur_player_strength) =
                player_groups.get_mut(&sec).expect("Confirmed to exist above");

            cur_total_time.time_sec += total_time_played.time_sec;
            cur_player_strength.0 += player_strength.0;

            if *player_next_pirate_spawn < *last_pirate_spawn {
                *last_pirate_spawn = *player_next_pirate_spawn;
            }

            ents.push(player_ent);
        } else {
            player_groups.insert(
                player_loc.sector,
                (*player_next_pirate_spawn, vec![player_ent], *total_time_played, *player_strength),
            );
        }
    }

    for (sector, (next_pirate_spawn, player_ents, total_time, player_strength)) in player_groups {
        if time.elapsed_secs_f64() < next_pirate_spawn.0 {
            continue;
        }

        let n_players = player_ents.len();

        let player_strength = PlayerStrength(player_strength.0 / n_players as f32);

        const MIN_SPAWN_DISTANCE: f32 = 5000.0;
        const MAX_SPAWN_TRIES: usize = 20;

        let mut fleet_origin = None;

        let mut itrs = 0;
        while fleet_origin.is_none() {
            if itrs >= MAX_SPAWN_TRIES {
                // give up before killing server
                break;
            }

            let spawn_sector = sector
                + Sector::new(
                    random_range(-1.0, 1.0).round() as SectorUnit,
                    random_range(-1.0, 1.0).round() as SectorUnit,
                    random_range(-1.0, 1.0).round() as SectorUnit,
                );

            // Don't spawn directly on top of players
            if spawn_sector == sector {
                continue;
            }

            let origin = Location::new(Vec3::new(random_coord(), random_coord(), random_coord()), spawn_sector);

            if q_players
                .iter()
                .any(|x| x.1.is_within_reasonable_range(&origin) && x.1.distance_sqrd(&origin) < MIN_SPAWN_DISTANCE * MIN_SPAWN_DISTANCE)
            {
                itrs += 1;
                continue;
            }

            fleet_origin = Some(origin);
        }

        if let Some(fleet_origin) = fleet_origin {
            const SPACING: f32 = 500.0;

            let mut difficulty_calculation = total_time.time_sec as f32 / TIME_DIFFICULTY_CONSTANT;
            difficulty_calculation += player_strength.0 * PLAYER_STRENGTH_INCREASE_FACTOR * n_players as f32;

            let mut total_difficulty_todo = difficulty_calculation.ceil() as u32;

            let mut p_idx: u32 = 0;
            while total_difficulty_todo > 0 {
                let offset = p_idx as f32 * SPACING;
                p_idx += 1;

                let loc_here = fleet_origin + Vec3::new(offset, 0.0, 0.0);

                let difficulty =
                    random_range(0.0, (total_difficulty_todo / p_idx.pow(2)).min(MAX_PIRATE_DIFFICULTY as u32) as f32).round() as u32;
                // Scale difficulty count w/ number already spawned, since more = way harder
                total_difficulty_todo -= total_difficulty_todo.min((difficulty + 1) * p_idx.pow(2));

                commands.spawn((
                    Name::new("Loading Pirate Ship"),
                    PirateNeedsSpawned {
                        location: loc_here,
                        difficulty,
                        heading_towards: Location::new(Vec3::ZERO, sector),
                    },
                ));
            }
        }

        let next_spawn_time = calculate_next_spawn_time(&time, &min_pirate_spawn_time);

        for player_ent in player_ents {
            commands.entity(player_ent).insert(NextPirateSpawn(next_spawn_time));
        }
    }
}

fn random_coord() -> f32 {
    random_range(-SECTOR_DIMENSIONS / 2.0, SECTOR_DIMENSIONS / 2.0)
}

#[derive(Resource, Reflect)]
struct MinPirateSpawnTime(Duration);

fn load_settings(mut commands: Commands) {
    commands.insert_resource(MinPirateSpawnTime(Duration::from_mins(30)));
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Pirate spawning should be done around this set
pub enum PirateSpawningSet {
    /// Entities with the [`PirateNeedsSpawned`] component will be spawn as a pirate
    PirateSpawningLogic,
}

fn calculate_next_spawn_time(time: &Time, min_pirate_spawn_time: &MinPirateSpawnTime) -> f64 {
    let min_secs = min_pirate_spawn_time.0.as_secs_f64();
    rand::random::<f64>() * min_secs * 3.0 + min_secs + time.elapsed_secs_f64()
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        FixedUpdate,
        PirateSpawningSet::PirateSpawningLogic
            .before(LoadingBlueprintSystemSet::BeginLoadingBlueprints)
            .run_if(in_state(GameState::Playing))
            .run_if(on_timer(Duration::from_secs(10))),
    )
    .add_systems(Startup, load_settings)
    .add_systems(
        FixedUpdate,
        (add_spawn_times, spawn_pirates, on_needs_pirate_spawned)
            .in_set(PirateSpawningSet::PirateSpawningLogic)
            .chain(),
    )
    .register_type::<NextPirateSpawn>();
}
