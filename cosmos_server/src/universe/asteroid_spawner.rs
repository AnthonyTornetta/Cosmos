//! Responsible for spawning planets near stars, but for now just spawns a planet at 0, 0, 0.

use std::time::Duration;

use bevy::{
    prelude::{in_state, App, Commands, Deref, DerefMut, IntoSystemConfigs, Query, Res, ResMut, Resource, Update, Vec3, With},
    time::common_conditions::on_timer,
    utils::HashSet,
};
use cosmos_core::{
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
    physics::location::{CosmosBundleSet, Location, Sector, SystemUnit, SECTOR_DIMENSIONS},
    structure::{
        asteroid::{asteroid_builder::TAsteroidBuilder, loading::AsteroidNeedsCreated, Asteroid, ASTEROID_LOAD_RADIUS},
        coordinates::ChunkCoordinate,
        full_structure::FullStructure,
        Structure,
    },
    universe::star::Star,
};
use rand::Rng;

use crate::{
    init::init_world::ServerSeed, persistence::is_sector_generated, rng::get_rng_for_sector, settings::ServerSettings, state::GameState,
    structure::asteroid::server_asteroid_builder::ServerAsteroidBuilder, universe::star::calculate_temperature_at,
};

use super::planet_spawner::is_planet_in_sector;

#[derive(Default, Resource, Deref, DerefMut)]
struct CachedSectors(HashSet<Sector>);

fn spawn_asteroid(
    query: Query<&Location, With<Asteroid>>,
    players: Query<&Location, With<Player>>,
    server_seed: Res<ServerSeed>,
    mut cache: ResMut<CachedSectors>,
    mut commands: Commands,
    q_stars: Query<(&Location, &Star)>,
    settings: Res<ServerSettings>,
) {
    if !settings.spawn_asteroids {
        return;
    }

    let mut to_check_sectors = HashSet::new();

    for l in players.iter() {
        for dsz in -(ASTEROID_LOAD_RADIUS as SystemUnit)..=ASTEROID_LOAD_RADIUS as SystemUnit {
            for dsy in -(ASTEROID_LOAD_RADIUS as SystemUnit)..=ASTEROID_LOAD_RADIUS as SystemUnit {
                for dsx in -(ASTEROID_LOAD_RADIUS as SystemUnit)..=ASTEROID_LOAD_RADIUS as SystemUnit {
                    let sector = l.sector() + Sector::new(dsx, dsy, dsz);
                    to_check_sectors.insert(sector);
                }
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
        let sector = loc.sector();
        cache.insert(sector);
        sectors.remove(&sector);
    }

    for sector in sectors {
        cache.insert(sector);

        if is_sector_generated(sector) || is_planet_in_sector(&sector, &server_seed) {
            // This sector has already been loaded, don't regenerate stuff
            continue;
        }

        let mut rng = get_rng_for_sector(&server_seed, &sector);

        if rng.gen_range(0..1000) < 100 {
            // Biased towards lower amounts
            let n_asteroids = (6.0 * (1.0 - (1.0 - rng.gen::<f32>()).sqrt())) as usize;

            let multiplier = SECTOR_DIMENSIONS;
            let adder = -SECTOR_DIMENSIONS / 2.0;

            let stars = q_stars.iter().map(|(x, y)| (*x, *y)).collect::<Vec<(Location, Star)>>();

            for _ in 0..n_asteroids {
                let size = rng.gen_range(4..=8);

                let loc = Location::new(
                    Vec3::new(
                        rng.gen::<f32>() * multiplier + adder,
                        rng.gen::<f32>() * multiplier + adder,
                        rng.gen::<f32>() * multiplier + adder,
                    ),
                    sector,
                );

                if let Some(temperature) = calculate_temperature_at(stars.iter(), &loc) {
                    let mut structure = Structure::Full(FullStructure::new(ChunkCoordinate::new(size, size, size)));
                    let builder = ServerAsteroidBuilder::default();
                    let mut entity_cmd = commands.spawn_empty();

                    builder.insert_asteroid(&mut entity_cmd, loc, &mut structure, temperature);

                    entity_cmd.insert((structure, AsteroidNeedsCreated));
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        spawn_asteroid
            .in_set(NetworkingSystemsSet::Between)
            .run_if(on_timer(Duration::from_secs(1)))
            .run_if(in_state(GameState::Playing)),
    )
    .insert_resource(CachedSectors::default());
}
