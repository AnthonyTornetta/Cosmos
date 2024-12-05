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
    reflect::Reflect,
    state::condition::in_state,
    time::{common_conditions::on_timer, Time},
    utils::hashbrown::HashMap,
};
use cosmos_core::{
    entities::player::Player,
    physics::location::{Location, Sector, SectorUnit, SECTOR_DIMENSIONS},
    state::GameState,
    utils::random::random_range,
};

use crate::{
    persistence::loading::{LoadingBlueprintSystemSet, NeedsBlueprintLoaded},
    settings::ServerSettings,
};

#[derive(Component)]
/// A pirate needs spawned for this entity, please add the components it needs to function
pub struct PirateNeedsSpawned(Location);

#[derive(Component)]
/// A pirate-controlled ship
pub struct Pirate;

const MAX_DIFFICULTY: u64 = 4;
const SECTORS_DIFFICULTY_INCREASE: u64 = 4;

fn on_needs_pirate_spawned(mut commands: Commands, q_needs_pirate_spawned: Query<(Entity, &PirateNeedsSpawned)>) {
    for (ent, pns) in q_needs_pirate_spawned.iter() {
        let difficulty = (pns.0.sector - Sector::new(25, 25, 25)).abs().max_element();
        let difficulty = (difficulty as u64 / SECTORS_DIFFICULTY_INCREASE).min(MAX_DIFFICULTY);

        commands.entity(ent).remove::<PirateNeedsSpawned>().insert((
            Pirate,
            NeedsBlueprintLoaded {
                path: format!("default_blueprints/pirate/default_{difficulty}.bp"),
                rotation: Quat::IDENTITY,
                spawn_at: pns.0,
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
    q_players: Query<(Entity, &Location, &NextPirateSpawn), With<Player>>,
    time: Res<Time>,
    min_pirate_spawn_time: Res<MinPirateSpawnTime>,
    server_settings: Res<ServerSettings>,
) {
    if server_settings.peaceful {
        return;
    }

    let mut player_groups: HashMap<Sector, (NextPirateSpawn, Vec<Entity>)> = HashMap::default();

    const MAX_DIST: f32 = SECTOR_DIMENSIONS * 2.0 + 20.0;

    for (player_ent, player_loc, &player_last_pirate_spawn) in q_players.iter() {
        if let Some(sec) = player_groups
            .keys()
            .find(|&sec| {
                player_loc.is_within_reasonable_range_sector(*sec)
                    && Location::new(Vec3::ZERO, *sec - player_loc.sector).distance_sqrd(&Location::ZERO) <= MAX_DIST * MAX_DIST
            })
            .copied()
        {
            let (last_pirate_spawn, ents) = player_groups.get_mut(&sec).expect("Confirmed to exist above");

            if player_last_pirate_spawn < *last_pirate_spawn {
                *last_pirate_spawn = player_last_pirate_spawn;
            }

            ents.push(player_ent);
        } else {
            player_groups.insert(player_loc.sector, (player_last_pirate_spawn, vec![player_ent]));
        }
    }

    for (sector, (next_pirate_spawn, player_ents)) in player_groups {
        if time.elapsed_secs_f64() < next_pirate_spawn.0 {
            continue;
        }

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

            let n_pirates = random_range(1.0, 4.0).round() as usize;

            for p_idx in 0..n_pirates {
                let offset = p_idx as f32 * SPACING;

                let loc_here = fleet_origin + Vec3::new(offset, 0.0, 0.0);

                commands.spawn((Name::new("Loading Pirate Ship"), PirateNeedsSpawned(loc_here)));
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

pub(super) fn register(app: &mut App) {
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
        (add_spawn_times, spawn_pirates, on_needs_pirate_spawned)
            .in_set(PirateSpawningSet::PirateSpawningLogic)
            .chain(),
    )
    .register_type::<NextPirateSpawn>();
}
