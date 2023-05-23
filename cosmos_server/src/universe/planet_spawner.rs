//! Responsible for spawning planets near stars, but for now just spawns a planet at 0, 0, 0.

use bevy::{
    prelude::{
        App, Commands, Component, Deref, DerefMut, DespawnRecursiveExt, Entity, IntoSystemConfigs,
        OnUpdate, Query, Res, ResMut, Resource, Vec3, With,
    },
    tasks::{AsyncComputeTaskPool, Task},
    utils::HashSet,
};
use cosmos_core::{
    entities::player::Player,
    physics::location::Location,
    structure::{
        planet::{planet_builder::TPlanetBuilder, Planet, PLANET_LOAD_RADIUS},
        Structure,
    },
    universe::star::Star,
};
use futures_lite::future;
use rand::Rng;

use crate::{
    init::init_world::ServerSeed, persistence::is_sector_loaded, rng::get_rng_for_sector,
    state::GameState, structure::planet::server_planet_builder::ServerPlanetBuilder,
};

#[derive(Debug, Default, Resource, Deref, DerefMut, Clone)]
struct CachedSectors(HashSet<(i64, i64, i64)>);

const BACKGROUND_TEMPERATURE: f32 = 50.0;
const TEMPERATURE_CONSTANT: f32 = 5.3e9;

#[derive(Component, Debug)]
struct PlanetSpawnerAsyncTask(Task<(CachedSectors, Vec<PlanetToSpawn>)>);

#[derive(Debug)]
struct PlanetToSpawn {
    temperature: f32,
    location: Location,
    size: usize,
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

            builder.insert_planet(
                &mut entity_cmd,
                loc,
                &mut structure,
                Planet::new(temperature),
            );

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
        cache.insert((l.sector_x, l.sector_y, l.sector_z));
    });

    let server_seed = server_seed.clone();
    let stars = stars
        .iter()
        .map(|(x, y)| (*x, *y))
        .collect::<Vec<(Location, Star)>>();

    let task = thread_pool.spawn(async move {
        let mut to_check_sectors = HashSet::new();

        for l in locs {
            for dsz in -(PLANET_LOAD_RADIUS as i64)..=(PLANET_LOAD_RADIUS as i64) {
                for dsy in -(PLANET_LOAD_RADIUS as i64)..=(PLANET_LOAD_RADIUS as i64) {
                    for dsx in -(PLANET_LOAD_RADIUS as i64)..=(PLANET_LOAD_RADIUS as i64) {
                        let sector = (dsx + l.sector_x, dsy + l.sector_y, dsz + l.sector_z);
                        if !cache.contains(&sector) {
                            to_check_sectors.insert(sector);
                        }
                    }
                }
            }
        }

        let mut made_stars = vec![];

        for (sx, sy, sz) in to_check_sectors {
            cache.insert((sx, sy, sz));

            if is_sector_loaded((sx, sy, sz)) {
                // This sector has already been loaded, don't regenerate stuff
                continue;
            }

            let mut rng = get_rng_for_sector(&server_seed, (sx, sy, sz));

            let is_origin = sx == 0 && sy == 0 && sz == 0;

            if is_origin || rng.gen_range(0..1000) == 9 {
                let location = Location::new(Vec3::ZERO, sx, sy, sz);

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
                    let size: usize = if is_origin {
                        50
                    } else {
                        rng.gen_range(200..=500)
                    };

                    let temperature = (TEMPERATURE_CONSTANT
                        * (star.temperature() / best_dist.unwrap()))
                    .max(BACKGROUND_TEMPERATURE);

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
pub fn is_planet_in_sector(sector: (i64, i64, i64), seed: &ServerSeed) -> bool {
    let mut rng: rand_chacha::ChaCha8Rng = get_rng_for_sector(&seed, sector);

    (sector.0 == 0 && sector.1 == 0 && sector.2 == 0) || rng.gen_range(0..1000) == 9
}

pub(super) fn register(app: &mut App) {
    app.add_systems((monitor_planets_to_spawn, spawn_planet).in_set(OnUpdate(GameState::Playing)))
        .insert_resource(CachedSectors::default());
}
