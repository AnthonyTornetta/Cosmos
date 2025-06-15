//! Spawns merchant ships

use std::time::Duration;

use bevy::{
    app::{App, Startup, Update},
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        query::{With, Without},
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res, Resource},
    },
    log::info,
    math::{Quat, Vec3},
    reflect::Reflect,
    state::condition::in_state,
    time::Time,
    platform::collections::HashMap,
};
use cosmos_core::{
    entities::player::Player,
    physics::location::{Location, SECTOR_DIMENSIONS, Sector, SectorUnit},
    state::GameState,
    utils::{quat_math::QuatMath, random::random_range},
};

use crate::{
    ai::quest_npc::MerchantFederation,
    entities::player::strength::{PlayerStrength, TotalTimePlayed},
    persistence::{
        loading::{LoadingBlueprintSystemSet, NeedsBlueprintLoaded},
        make_persistent::make_persistent,
    },
    settings::ServerSettings,
};

/// TODO: Load this from config
///
/// Total playtime (sec) is divided by this to calculate its difficulty.
const TIME_DIFFICULTY_CONSTANT: f32 = 80_000.0;

/// TODO: Load this from config
///
/// How much one player's strength will increase the merchants's difficulty.
const PLAYER_STRENGTH_INCREASE_FACTOR: f32 = 1.0;

#[derive(Component)]
/// A merchant needs spawned for this entity, please add the components it needs to function
pub struct MerchantNeedsSpawned {
    location: Location,
    difficulty: u32,
    heading_towards: Location,
}

/// The maximum difficulty of ship we can spawn. This is NOT the total difficulty.
const MAX_DIFFICULTY: u64 = 0;

fn on_needs_merchant_spawned(mut commands: Commands, q_needs_merchant_spawned: Query<(Entity, &MerchantNeedsSpawned)>) {
    for (ent, pns) in q_needs_merchant_spawned.iter() {
        let difficulty = pns.difficulty;

        let rotation = (pns.heading_towards - pns.location).absolute_coords_f32().normalize_or_zero();

        commands.entity(ent).remove::<MerchantNeedsSpawned>().insert((
            MerchantFederation,
            NeedsBlueprintLoaded {
                path: format!("default_blueprints/merchant/default_{difficulty}.bp"),
                rotation: Quat::looking_to(rotation, Vec3::Y),
                spawn_at: pns.location,
            },
        ));
    }
}

#[derive(Component, Clone, Copy, PartialEq, PartialOrd, Reflect, Debug)]
/// Goes on the player and indicates the next time a merchant ship will spawn.
///
/// When this number hits 0.0, spawn a merchant ship.
struct NextMerchantSpawn(f64);

fn add_spawn_times(
    q_players: Query<Entity, (With<Player>, Without<NextMerchantSpawn>)>,
    min_merchant_spawn_time: Res<FirstMerchantSpawnTime>,
    mut commands: Commands,
) {
    for ent in q_players.iter() {
        let next_spawn_time = calculate_next_spawn_time(min_merchant_spawn_time.0);
        commands.entity(ent).insert(NextMerchantSpawn(next_spawn_time));
    }
}

fn spawn_merchant_ships(
    mut commands: Commands,
    mut q_players: Query<(Entity, &Location, &mut NextMerchantSpawn, &TotalTimePlayed, &PlayerStrength), With<Player>>,
    time: Res<Time>,
    min_merchant_spawn_time: Res<MinMerchantSpawnTime>,
    server_settings: Res<ServerSettings>,
) {
    if !server_settings.spawn_merchant_ships {
        return;
    }

    let mut player_groups: HashMap<Sector, (NextMerchantSpawn, Vec<Entity>, TotalTimePlayed, PlayerStrength)> = HashMap::default();

    const MAX_DIST: f32 = SECTOR_DIMENSIONS * 2.0 + 20.0;

    for (player_ent, player_loc, mut next_merchant_spawn, total_time_played, player_strength) in q_players.iter_mut() {
        next_merchant_spawn.0 = (next_merchant_spawn.0 - time.delta().as_secs_f64()).max(0.0);

        let next_merchant_spawn = *next_merchant_spawn;

        if let Some(sec) = player_groups
            .keys()
            .find(|&sec| {
                player_loc.is_within_reasonable_range_sector(*sec)
                    && Location::new(Vec3::ZERO, *sec - player_loc.sector).distance_sqrd(&Location::ZERO) <= MAX_DIST * MAX_DIST
            })
            .copied()
        {
            let (next_merchant_spawn_time, ents, _, cur_player_strength) = player_groups.get_mut(&sec).expect("Confirmed to exist above");

            cur_player_strength.0 += player_strength.0;

            if next_merchant_spawn < *next_merchant_spawn_time {
                *next_merchant_spawn_time = next_merchant_spawn;
            }

            ents.push(player_ent);
        } else {
            player_groups.insert(
                player_loc.sector,
                (next_merchant_spawn, vec![player_ent], *total_time_played, *player_strength),
            );
        }
    }

    for (sector, (next_merchant_spawn, player_ents, total_time, player_strength)) in player_groups {
        if next_merchant_spawn.0 != 0.0 {
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

                let difficulty = random_range(0.0, (total_difficulty_todo / p_idx.pow(2)).min(MAX_DIFFICULTY as u32) as f32).round() as u32;
                // Scale difficulty count w/ number already spawned, since more = way harder
                total_difficulty_todo -= total_difficulty_todo.min((difficulty + 1) * p_idx.pow(2));

                info!("Loading thing");

                commands.spawn((
                    Name::new("Loading Merchant Ship"),
                    MerchantNeedsSpawned {
                        location: loc_here,
                        difficulty,
                        heading_towards: Location::new(Vec3::ZERO, sector),
                    },
                ));
            }
        }

        let next_spawn_time = calculate_next_spawn_time(min_merchant_spawn_time.0);

        for player_ent in player_ents {
            commands.entity(player_ent).insert(NextMerchantSpawn(next_spawn_time));
        }
    }
}

fn random_coord() -> f32 {
    random_range(-SECTOR_DIMENSIONS / 2.0, SECTOR_DIMENSIONS / 2.0)
}

#[derive(Resource, Reflect)]
struct MinMerchantSpawnTime(Duration);

#[derive(Resource, Reflect)]
struct FirstMerchantSpawnTime(Duration);

fn load_settings(mut commands: Commands) {
    commands.insert_resource(MinMerchantSpawnTime(Duration::from_mins(40)));
    commands.insert_resource(FirstMerchantSpawnTime(Duration::from_mins(20)));
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum MerchantSpawningSet {
    MerchantSpawningLogic,
}

fn calculate_next_spawn_time(min_merchant_spawn_time: Duration) -> f64 {
    let min_secs = min_merchant_spawn_time.as_secs_f64();
    rand::random::<f64>() * min_secs * 0.5 + min_secs
}

pub(super) fn register(app: &mut App) {
    make_persistent::<TotalTimePlayed>(app);
    make_persistent::<PlayerStrength>(app);

    app.configure_sets(
        Update,
        MerchantSpawningSet::MerchantSpawningLogic
            .before(LoadingBlueprintSystemSet::BeginLoadingBlueprints)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(Startup, load_settings)
    .add_systems(
        Update,
        (add_spawn_times, spawn_merchant_ships, on_needs_merchant_spawned)
            .in_set(MerchantSpawningSet::MerchantSpawningLogic)
            .chain(),
    )
    .register_type::<FirstMerchantSpawnTime>()
    .register_type::<NextMerchantSpawn>()
    .register_type::<PlayerStrength>()
    .register_type::<TotalTimePlayed>();
}
