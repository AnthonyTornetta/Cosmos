//! Responsible for spawning planets near stars, but for now just spawns a planet at 0, 0, 0.

use bevy::{
    prelude::{
        in_state, App, Commands, Deref, DerefMut, IntoSystemConfig, Query, Res, ResMut, Resource,
        Vec3, With,
    },
    utils::HashSet,
};
use cosmos_core::{
    entities::player::Player,
    physics::location::Location,
    structure::{
        planet::{planet_builder::TPlanetBuilder, Planet, PLANET_UNLOAD_RADIUS},
        Structure,
    },
    universe::star::Star,
};
use rand::Rng;

use crate::{
    init::init_world::ServerSeed, persistence::is_sector_loaded, rng::get_rng_for_sector,
    state::GameState, structure::planet::server_planet_builder::ServerPlanetBuilder,
};

#[derive(Default, Resource, Deref, DerefMut)]
struct CachedSectors(HashSet<(i64, i64, i64)>);

const BACKGROUND_TEMPERATURE: f32 = 50.0;
const TEMPERATURE_CONSTANT: f32 = 5.3e9;

#[derive(Resource, Debug)]
/// Used to not check everything at once (too intensive), but rather divide
/// the area it checks into multiple quadrants it can check individually
struct Quadrant(f32, f32, f32);

const SUBDIVISIONS: f32 = 8.0;

fn spawn_planet(
    query: Query<&Location, With<Planet>>,
    players: Query<&Location, With<Player>>,
    server_seed: Res<ServerSeed>,
    mut cache: ResMut<CachedSectors>,
    mut commands: Commands,
    stars: Query<(&Location, &Star), With<Star>>,
    mut quadrant: ResMut<Quadrant>,
) {
    let mut to_check_sectors = HashSet::new();

    for l in players.iter() {
        for dsz in -(((1.0 - quadrant.2 / SUBDIVISIONS) * PLANET_UNLOAD_RADIUS as f32) as i64)
            ..=((quadrant.2 / SUBDIVISIONS * PLANET_UNLOAD_RADIUS as f32) as i64)
        {
            for dsy in -(((1.0 - quadrant.1 / SUBDIVISIONS) * PLANET_UNLOAD_RADIUS as f32) as i64)
                ..=((quadrant.1 / SUBDIVISIONS * PLANET_UNLOAD_RADIUS as f32) as i64)
            {
                for dsx in -(((1.0 - quadrant.0 / SUBDIVISIONS) * PLANET_UNLOAD_RADIUS as f32)
                    as i64)
                    ..=((quadrant.0 / SUBDIVISIONS * PLANET_UNLOAD_RADIUS as f32) as i64)
                {
                    let sector = (dsx + l.sector_x, dsy + l.sector_y, dsz + l.sector_z);
                    to_check_sectors.insert(sector);
                }
            }
        }
    }

    quadrant.0 += 1.0;
    if quadrant.0 > SUBDIVISIONS {
        quadrant.0 = 0.0;

        quadrant.1 += 1.0;
        if quadrant.1 > SUBDIVISIONS {
            quadrant.1 = 0.0;

            quadrant.2 += 1.0;
            if quadrant.2 > SUBDIVISIONS {
                quadrant.2 = 0.0;
            }
        }
    }

    let mut dead_sectors = HashSet::new();

    // Clear out unloaded sectors from the cache
    for sector in cache.iter() {
        if !to_check_sectors.contains(sector) {
            dead_sectors.insert(*sector);
        }
    }

    for dead_sector in dead_sectors {
        cache.remove(&dead_sector);
    }

    let mut sectors = HashSet::new();

    for sector in to_check_sectors {
        if !cache.contains(&sector) {
            sectors.insert(sector);
        }
    }

    for loc in query.iter() {
        let sector = (loc.sector_x, loc.sector_y, loc.sector_z);
        cache.insert(sector);
        sectors.remove(&sector);
    }

    for (sx, sy, sz) in sectors {
        cache.insert((sx, sy, sz));

        if is_sector_loaded((sx, sy, sz)) {
            // This sector has already been loaded, don't regenerate stuff
            continue;
        }

        let mut rng = get_rng_for_sector(&server_seed, (sx, sy, sz));

        let is_origin = sx == 0 && sy == 0 && sz == 0;

        if is_origin || rng.gen_range(0..1000) == 99999 {
            let loc = Location::new(Vec3::ZERO, sx, sy, sz);

            let mut closest_star = None;
            let mut best_dist = None;

            for (star_loc, star) in stars.iter() {
                let dist = loc.distance_sqrd(star_loc);

                if closest_star.is_none() || best_dist.unwrap() < dist {
                    closest_star = Some(star);
                    best_dist = Some(dist);
                }
            }

            if let Some(star) = closest_star {
                let mut entity_cmd = commands.spawn_empty();

                let size: usize = if is_origin {
                    50
                } else {
                    rng.gen_range(200..=500)
                };

                let mut structure = Structure::new(size, size, size);

                let builder = ServerPlanetBuilder::default();

                let temperature = (TEMPERATURE_CONSTANT
                    * (star.temperature() / best_dist.unwrap()))
                .max(BACKGROUND_TEMPERATURE);

                builder.insert_planet(
                    &mut entity_cmd,
                    loc,
                    &mut structure,
                    Planet::new(temperature),
                );

                entity_cmd.insert(structure);
            }
        }
    }
}

/// Checks if there should be a planet in this sector.
pub fn is_planet_in_sector(sector: (i64, i64, i64), seed: &ServerSeed) -> bool {
    let mut rng: rand_chacha::ChaCha8Rng = get_rng_for_sector(&seed, sector);

    (sector.0 == 0 && sector.1 == 0 && sector.2 == 0) || rng.gen_range(0..1000) == 9
}

pub(super) fn register(app: &mut App) {
    app.add_system(spawn_planet.run_if(in_state(GameState::Playing)))
        .insert_resource(CachedSectors::default())
        .insert_resource(Quadrant(0.0, 0.0, 0.0));
}
