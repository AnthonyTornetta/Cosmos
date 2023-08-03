//! Responsible for spawning planets near stars, but for now just spawns a planet at 0, 0, 0.

use std::time::Duration;

use bevy::{
    prelude::{
        in_state, App, Commands, Component, Deref, DerefMut, DespawnRecursiveExt, Entity, IntoSystemConfigs, Query, Res, ResMut, Resource,
        Update, Vec3, With,
    },
    tasks::{AsyncComputeTaskPool, Task},
    time::common_conditions::on_timer,
    utils::HashSet,
};
use cosmos_core::{
    entities::player::Player,
    physics::location::{Location, Sector, SystemUnit},
    structure::{
        coordinates::CoordinateType,
        planet::{planet_builder::TPlanetBuilder, Planet, PLANET_LOAD_RADIUS},
        Structure,
    },
    universe::star::Star,
};
use futures_lite::future;
use rand::Rng;

use crate::{
    init::init_world::ServerSeed, persistence::is_sector_loaded, rng::get_rng_for_sector, state::GameState,
    structure::planet::server_planet_builder::ServerPlanetBuilder,
};

#[derive(Debug, Default, Resource, Deref, DerefMut, Clone)]
struct CachedSectors(HashSet<Sector>);

const BACKGROUND_TEMPERATURE: f32 = 50.0;
const TEMPERATURE_CONSTANT: f32 = 5.3e9;

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

            let mut structure = Structure::new(size, size, size);

            let builder = ServerPlanetBuilder::default();

            builder.insert_planet(&mut entity_cmd, loc, &mut structure, Planet::new(temperature));

            entity_cmd.insert(structure);
        }

        *sectors_cache = cache;
    }
}

fn spawn_planet(
    query: Query<&Location, With<Planet>>,
    players: Query<&Location, With<Player>>,
    server_seed: Res<ServerSeed>,
    mut commands: Commands,
    stars: Query<(&Location, &Star), With<Star>>,
    cache: Res<CachedSectors>,
    is_already_generating: Query<(), With<PlanetSpawnerAsyncTask>>,
) {
    if !is_already_generating.is_empty() {
        // an async task is already running, don't make another one
        return;
    }

    let thread_pool = AsyncComputeTaskPool::get();

    let locs = players.iter().copied().collect::<Vec<Location>>();

    let mut cache = cache.clone();

    query.iter().for_each(|l| {
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

            if is_sector_loaded(sector) {
                // This sector has already been loaded, don't regenerate stuff
                continue;
            }

            let mut rng = get_rng_for_sector(&server_seed, &sector);

            let is_origin = sector.x() == 25 && sector.y() == 25 && sector.z() == 25;

            if is_origin || rng.gen_range(0..1000) == 9 {
                let location = Location::new(Vec3::ZERO, sector);

                let mut closest_star = None;
                let mut best_dist = None;

                for (star_loc, star) in stars.iter() {
                    let dist = location.distance_sqrd(star_loc);

                    if closest_star.is_none() || best_dist.unwrap() < dist {
                        closest_star = Some(star);
                        best_dist = Some(dist);
                    }
                }

                if let Some(star) = closest_star {
                    let size = if is_origin { 50 } else { rng.gen_range(200..=500) };

                    let distance_scaling = best_dist.expect("This would have been set at this point.") / 2.0;

                    let temperature = (TEMPERATURE_CONSTANT * (star.temperature() / distance_scaling)).max(BACKGROUND_TEMPERATURE);

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

    commands.spawn(PlanetSpawnerAsyncTask(task));
}

/// Checks if there should be a planet in this sector.
pub fn is_planet_in_sector(sector: &Sector, seed: &ServerSeed) -> bool {
    let mut rng: rand_chacha::ChaCha8Rng = get_rng_for_sector(seed, sector);

    (sector.x() == 0 && sector.y() == 0 && sector.z() == 0) || rng.gen_range(0..1000) == 9
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (monitor_planets_to_spawn, spawn_planet.run_if(on_timer(Duration::from_millis(1000)))).run_if(in_state(GameState::Playing)),
    )
    .insert_resource(CachedSectors::default());
}
