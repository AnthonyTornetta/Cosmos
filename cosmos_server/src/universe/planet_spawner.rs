//! Responsible for spawning planets near stars, but for now just spawns a planet at 0, 0, 0.

use std::time::Duration;

use bevy::{
    core::Name,
    math::Quat,
    prelude::{
        in_state, App, Commands, Component, Deref, DerefMut, DespawnRecursiveExt, Entity, IntoSystemConfigs, Query, Res, ResMut, Resource,
        Update, Vec3, With,
    },
    tasks::{AsyncComputeTaskPool, Task},
    time::common_conditions::on_timer,
    utils::HashSet,
};
use cosmos_core::{
    ecs::bundles::BundleStartingRotation,
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
    physics::location::{Location, Sector, SystemUnit},
    structure::{
        coordinates::CoordinateType,
        dynamic_structure::DynamicStructure,
        planet::{planet_builder::TPlanetBuilder, Planet, PLANET_LOAD_RADIUS},
        Structure,
    },
    universe::star::Star,
};
use futures_lite::future;
use rand::Rng;

use crate::{
    init::init_world::ServerSeed, persistence::is_sector_generated, rng::get_rng_for_sector, settings::ServerSettings, state::GameState,
    structure::planet::server_planet_builder::ServerPlanetBuilder,
};

use super::star::calculate_temperature_at;

#[derive(Debug, Default, Resource, Deref, DerefMut, Clone)]
struct CachedSectors(HashSet<Sector>);

#[derive(Component, Debug)]
struct PlanetSpawnerAsyncTask(Task<(CachedSectors, Vec<PlanetToSpawn>)>);

#[derive(Debug)]
struct PlanetToSpawn {
    temperature: f32,
    location: Location,
    size: CoordinateType,
}

fn monitor_planets_to_spawn(
    mut query: Query<(Entity, &mut PlanetSpawnerAsyncTask)>,
    mut commands: Commands,
    mut sectors_cache: ResMut<CachedSectors>,
) {
    let Ok((entity, mut task)) = query.get_single_mut() else {
        return;
    };

    if let Some((cache, planets)) = future::block_on(future::poll_once(&mut task.0)) {
        commands.entity(entity).despawn_recursive();

        for planet in planets {
            let (size, loc, temperature) = (planet.size, planet.location, planet.temperature);

            let mut entity_cmd = commands.spawn_empty();

            let mut structure = Structure::Dynamic(DynamicStructure::new(size));

            let builder = ServerPlanetBuilder::default();

            builder.insert_planet(&mut entity_cmd, loc, &mut structure, Planet::new(temperature));

            entity_cmd.insert((structure, BundleStartingRotation(Quat::from_axis_angle(Vec3::X, 0.9))));
        }

        *sectors_cache = cache;
    }
}

fn spawn_planet(
    q_planet_locations: Query<&Location, With<Planet>>,
    q_player_locations: Query<&Location, With<Player>>,
    server_seed: Res<ServerSeed>,
    mut commands: Commands,
    stars: Query<(&Location, &Star), With<Star>>,
    cache: Res<CachedSectors>,
    is_already_generating: Query<(), With<PlanetSpawnerAsyncTask>>,
    server_settings: Res<ServerSettings>,
) {
    if !server_settings.spawn_planets {
        return;
    }

    if !is_already_generating.is_empty() {
        // an async task is already running, don't make another one
        return;
    }

    if q_player_locations.is_empty() {
        // Don't bother if there are no players
        return;
    }

    let thread_pool = AsyncComputeTaskPool::get();

    let locs = q_player_locations.iter().copied().collect::<Vec<Location>>();

    let mut cache = cache.clone();

    q_planet_locations.iter().for_each(|l| {
        cache.insert(l.sector());
    });

    let server_seed = *server_seed;
    let stars = stars.iter().map(|(x, y)| (*x, *y)).collect::<Vec<(Location, Star)>>();

    let task = thread_pool.spawn(async move {
        let mut to_check_sectors = HashSet::new();

        for l in locs {
            for dsz in -(PLANET_LOAD_RADIUS as SystemUnit)..=(PLANET_LOAD_RADIUS as SystemUnit) {
                for dsy in -(PLANET_LOAD_RADIUS as SystemUnit)..=(PLANET_LOAD_RADIUS as SystemUnit) {
                    for dsx in -(PLANET_LOAD_RADIUS as SystemUnit)..=(PLANET_LOAD_RADIUS as SystemUnit) {
                        let sector = l.sector() + Sector::new(dsx, dsy, dsz);
                        if !cache.contains(&sector) {
                            to_check_sectors.insert(sector);
                        }
                    }
                }
            }
        }

        let mut made_stars = vec![];

        for sector in to_check_sectors {
            cache.insert(sector);

            if is_sector_generated(sector) {
                // This sector has already been loaded, don't regenerate stuff
                continue;
            }

            let mut rng = get_rng_for_sector(&server_seed, &sector);

            let is_origin = sector.x() == 25 && sector.y() == 25 && sector.z() == 25;

            if is_origin || rng.gen_range(0..1000) == 9 {
                let location = Location::new(Vec3::ZERO, sector);

                if let Some(temperature) = calculate_temperature_at(stars.iter(), &location) {
                    let size = if is_origin {
                        64
                    } else {
                        2_f32.powi(rng.gen_range(7..=9)) as CoordinateType
                    };

                    made_stars.push(PlanetToSpawn {
                        size,
                        temperature,
                        location,
                    });
                }
            }
        }

        (cache, made_stars)
    });

    commands.spawn((Name::new("Planet spawner async task"), PlanetSpawnerAsyncTask(task)));
}

/// Checks if there should be a planet in this sector.
pub fn is_planet_in_sector(sector: &Sector, seed: &ServerSeed) -> bool {
    let mut rng: rand_chacha::ChaCha8Rng = get_rng_for_sector(seed, sector);

    /*(sector.x() == 0 && sector.y() == 0 && sector.z() == 0) || */
    rng.gen_range(0..1000) == 9
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (monitor_planets_to_spawn, spawn_planet.run_if(on_timer(Duration::from_millis(1000))))
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    )
    .insert_resource(CachedSectors::default());
}
