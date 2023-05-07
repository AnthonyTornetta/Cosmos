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
};
use rand::Rng;

use crate::{
    init::init_world::ServerSeed, persistence::is_sector_loaded, rng::get_rng_for_sector,
    state::GameState, structure::planet::server_planet_builder::ServerPlanetBuilder,
};

#[derive(Default, Resource, Deref, DerefMut)]
struct CachedSectors(HashSet<(i64, i64, i64)>);

fn spawn_planet(
    query: Query<&Location, With<Planet>>,
    players: Query<&Location, With<Player>>,
    server_seed: Res<ServerSeed>,
    mut cache: ResMut<CachedSectors>,
    mut commands: Commands,
) {
    let mut to_check_sectors = HashSet::new();

    for l in players.iter() {
        let range = -(PLANET_UNLOAD_RADIUS as i64)..=(PLANET_UNLOAD_RADIUS as i64);
        for dsz in range.clone() {
            for dsy in range.clone() {
                for dsx in range.clone() {
                    let sector = (dsx + l.sector_x, dsy + l.sector_y, dsz + l.sector_z);
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

        if is_origin || rng.gen_range(0..1000) == 9 {
            let loc = Location::new(Vec3::ZERO, sx, sy, sz);

            let mut entity_cmd = commands.spawn_empty();

            let size: usize = if is_origin {
                10
            } else {
                rng.gen_range(200..=500)
            };

            let mut structure = Structure::new(size, size, size);

            let builder = ServerPlanetBuilder::default();

            builder.insert_planet(&mut entity_cmd, loc, &mut structure, Planet::new(100.0));

            entity_cmd.insert(structure);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(spawn_planet.run_if(in_state(GameState::Playing)))
        .insert_resource(CachedSectors::default());
}
