//! Spawns pirate ships

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
    math::{Quat, Vec3},
    prelude::{Added, EventReader},
    reflect::Reflect,
    state::condition::in_state,
    time::{common_conditions::on_timer, Time},
    utils::hashbrown::HashMap,
};
use cosmos_core::{
    block::block_events::BlockEventsSet,
    entities::player::Player,
    netty::{sync::IdentifiableComponent, system_sets::NetworkingSystemsSet},
    physics::location::{Location, Sector, SectorUnit, SECTOR_DIMENSIONS},
    state::GameState,
    structure::{block_health::events::BlockTakeDamageEvent, shared::MeltingDown, ship::pilot::Pilot},
    utils::random::random_range,
};
use serde::{Deserialize, Serialize};

use crate::{
    persistence::{
        loading::{LoadingBlueprintSystemSet, LoadingSystemSet, NeedsBlueprintLoaded},
        make_persistent::{make_persistent, PersistentComponent},
    },
    settings::ServerSettings,
};

/// TODO: Load this from config
///
/// Total playtime (sec) is divided by this to calculate its difficulty.
const TIME_DIFFICULTY_CONSTANT: f32 = 80_000.0;

/// TODO: Load this from config
///
/// How much one player's strength will increase the pirate's difficulty.
const PLAYER_STRENGTH_INCREASE_FACTOR: f32 = 1.0;

/// TODO: Load this from config
///
/// How much killing a pirate will increase the difficulty.
/// Aka, if you do 100% of the damage, your strength percentage will increase by this percent.
const DIFFICULTY_INCREASE: f32 = 5.0;

#[derive(Component)]
/// A pirate needs spawned for this entity, please add the components it needs to function
pub struct PirateNeedsSpawned {
    location: Location,
    difficulty: u32,
}

#[derive(Component)]
/// A pirate-controlled ship
pub struct Pirate;

/// The maximum difficulty of ship we can spawn. This is NOT the total difficulty.
const MAX_DIFFICULTY: u64 = 3;

#[derive(Component, Reflect, Debug, Clone, Copy, Default, Serialize, Deserialize)]
/// Represents how the enemies perceive your strength as a percentage between 0.0 and 100.0.
///
/// At 0.0%, the enemies will send their weakest fighters at you. At 100.0%, enemies will send
/// their most advanced fleets at you.
///
/// Killing pirates increases your stength, and dying lowers it.
struct PlayerStrength(f32);

impl IdentifiableComponent for PlayerStrength {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:player_strength"
    }
}

impl PersistentComponent for PlayerStrength {}

#[derive(Component, Reflect, Debug, Clone, Copy, Default, Serialize, Deserialize)]
/// Represents the total time a player has played on the server
///
/// Used for difficulty calculations
struct TotalTimePlayed {
    pub time_sec: u64,
}

impl IdentifiableComponent for TotalTimePlayed {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:total_time_played"
    }
}

impl PersistentComponent for TotalTimePlayed {}

fn on_needs_pirate_spawned(mut commands: Commands, q_needs_pirate_spawned: Query<(Entity, &PirateNeedsSpawned)>) {
    for (ent, pns) in q_needs_pirate_spawned.iter() {
        let difficulty = pns.difficulty;

        commands.entity(ent).remove::<PirateNeedsSpawned>().insert((
            Pirate,
            NeedsBlueprintLoaded {
                path: format!("default_blueprints/pirate/default_{difficulty}.bp"),
                rotation: Quat::IDENTITY,
                spawn_at: pns.location,
            },
        ));
    }
}

#[derive(Component, Clone, Copy, PartialEq, PartialOrd, Reflect, Debug)]
/// Goes on the player and ensures they don't deal with too many pirates
struct NextPirateSpawn(f64);

fn add_spawn_times(q_players: Query<Entity, (With<Player>, Without<NextPirateSpawn>)>, time: Res<Time>, mut commands: Commands) {
    for ent in q_players.iter() {
        commands.entity(ent).insert(NextPirateSpawn(time.delta_secs_f64()));
    }
}

fn spawn_pirates(
    mut commands: Commands,
    q_players: Query<(Entity, &Location, &NextPirateSpawn, &TotalTimePlayed, &PlayerStrength), With<Player>>,
    time: Res<Time>,
    min_pirate_spawn_time: Res<MinPirateSpawnTime>,
    server_settings: Res<ServerSettings>,
) {
    if server_settings.peaceful {
        return;
    }

    let mut player_groups: HashMap<Sector, (NextPirateSpawn, Vec<Entity>, TotalTimePlayed, PlayerStrength)> = HashMap::default();

    const MAX_DIST: f32 = SECTOR_DIMENSIONS * 2.0 + 20.0;

    for (player_ent, player_loc, &player_last_pirate_spawn, total_time_played, player_strength) in q_players.iter() {
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

            if player_last_pirate_spawn < *last_pirate_spawn {
                *last_pirate_spawn = player_last_pirate_spawn;
            }

            ents.push(player_ent);
        } else {
            player_groups.insert(
                player_loc.sector,
                (player_last_pirate_spawn, vec![player_ent], *total_time_played, *player_strength),
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

            let origin = Location::new(
                Vec3::new(random_coord(), random_coord(), random_coord()),
                sector
                    + Sector::new(
                        random_range(-1.0, 1.0).round() as SectorUnit,
                        random_range(-1.0, 1.0).round() as SectorUnit,
                        random_range(-1.0, 1.0).round() as SectorUnit,
                    ),
            );

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

                commands.spawn((
                    Name::new("Loading Pirate Ship"),
                    PirateNeedsSpawned {
                        location: loc_here,
                        difficulty,
                    },
                ));
            }
        }

        let min_secs = min_pirate_spawn_time.0.as_secs_f64();
        let next_spawn_time = rand::random::<f64>() * min_secs * 3.0 + min_secs + time.elapsed_secs_f64();

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
enum PirateSpawningSet {
    PirateSpawningLogic,
}

fn add_player_strength(mut commands: Commands, q_needs_player_strength: Query<Entity, (Added<Player>, Without<PlayerStrength>)>) {
    for ent in q_needs_player_strength.iter() {
        commands.entity(ent).insert(PlayerStrength::default());
    }
}

fn add_total_time_played(mut commands: Commands, q_needs_total_played: Query<Entity, (Added<Player>, Without<TotalTimePlayed>)>) {
    for ent in q_needs_total_played.iter() {
        commands.entity(ent).insert(TotalTimePlayed::default());
    }
}

fn advance_total_time(mut q_total_time: Query<&mut TotalTimePlayed>) {
    for mut tt in q_total_time.iter_mut() {
        tt.time_sec += 1;
    }
}

#[derive(Component, Default, Reflect, Debug)]
struct Hitters(HashMap<Entity, u64>);

fn process_hit_events(
    mut q_pirate: Query<&mut Hitters, With<Pirate>>,
    q_pilot: Query<&Pilot>,
    mut evr_hit_block: EventReader<BlockTakeDamageEvent>,
) {
    for ev in evr_hit_block.read() {
        let Some(causer) = ev.causer else {
            continue;
        };

        let causer = q_pilot.get(causer).map(|x| x.entity).unwrap_or(causer);

        let Ok(mut hitters) = q_pirate.get_mut(ev.structure_entity) else {
            continue;
        };

        *hitters.as_mut().0.entry(causer).or_default() += 1;
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

fn add_hitters(mut commands: Commands, q_needs_hitter: Query<Entity, (With<Pirate>, Without<Hitters>)>) {
    for ent in q_needs_hitter.iter() {
        commands.entity(ent).insert(Hitters::default());
    }
}

fn on_melt_down(mut q_players: Query<&mut PlayerStrength>, q_melting_down: Query<&Hitters, Added<MeltingDown>>) {
    for hitters in q_melting_down.iter() {
        let dmg_total = hitters.0.iter().map(|(_, hits)| *hits).sum::<u64>();

        for (&hitter_ent, &hits) in hitters.0.iter() {
            let percent = hits as f32 / dmg_total as f32;
            let Ok(mut player_strength) = q_players.get_mut(hitter_ent) else {
                continue;
            };

            player_strength.0 += percent * DIFFICULTY_INCREASE;
            player_strength.0 = player_strength.0.clamp(0.0, 100.0);
        }
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<TotalTimePlayed>(app);
    make_persistent::<PlayerStrength>(app);

    app.configure_sets(
        Update,
        PirateSpawningSet::PirateSpawningLogic
            .before(LoadingBlueprintSystemSet::BeginLoadingBlueprints)
            .run_if(in_state(GameState::Playing))
            .run_if(on_timer(Duration::from_secs(10))),
    )
    .add_systems(Startup, load_settings)
    .add_systems(
        Update,
        (add_spawn_times, spawn_pirates, on_needs_pirate_spawned, add_hitters)
            .in_set(PirateSpawningSet::PirateSpawningLogic)
            .chain(),
    )
    .add_systems(
        Update,
        process_hit_events
            .after(add_total_time_played)
            .after(advance_total_time)
            .after(tick_down_hitters)
            .in_set(BlockEventsSet::ProcessEvents),
    )
    .add_systems(Update, on_melt_down.after(process_hit_events).in_set(NetworkingSystemsSet::Between))
    .add_systems(
        Update,
        (add_total_time_played, add_player_strength).after(LoadingSystemSet::DoneLoading),
    )
    .add_systems(Update, advance_total_time.run_if(on_timer(Duration::from_secs(1))))
    .add_systems(Update, tick_down_hitters.run_if(on_timer(Duration::from_secs(1))))
    .register_type::<Hitters>()
    .register_type::<NextPirateSpawn>()
    .register_type::<PlayerStrength>()
    .register_type::<TotalTimePlayed>();
}
