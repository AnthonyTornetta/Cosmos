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
    persistence::LoadingDistance,
    physics::location::Location,
    structure::{
        planet::{planet_builder::TPlanetBuilder, Planet, PLANET_LOAD_RADIUS},
        Structure,
    },
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::{
    init::init_world::ServerSeed,
    persistence::is_sector_loaded,
    state::GameState,
    structure::planet::{
        biosphere::{grass_biosphere::GrassBiosphere, TBiosphere},
        server_planet_builder::ServerPlanetBuilder,
    },
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
    let mut sectors = HashSet::new();

    for l in players.iter() {
        let range = -(PLANET_LOAD_RADIUS as i64)..=(PLANET_LOAD_RADIUS as i64);
        for dsz in range.clone() {
            for dsy in range.clone() {
                for dsx in range.clone() {
                    let sector = (dsx + l.sector_x, dsy + l.sector_y, dsz + l.sector_z);
                    if !cache.contains(&sector) {
                        sectors.insert(sector);
                    }
                }
            }
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
            continue;
        }

        let rng = ChaCha8Rng::seed_from_u64(
            (server_seed.as_u64() as i64)
                .wrapping_add(sx)
                .wrapping_mul(sy)
                .wrapping_add(sy)
                .wrapping_mul(sx)
                .wrapping_add(sy)
                .wrapping_mul(sz)
                .wrapping_add(sz)
                .abs() as u64,
        )
        .gen_range(0..1000);

        if (sx == 0 && sy == 0 && sz == 0) || rng == 9 {
            println!("Genned {rng} for {sx} {sy} {sz}");
            let loc = Location::new(Vec3::ZERO, sx, sy, sz);

            let mut entity_cmd = commands.spawn_empty();

            let mut structure = Structure::new(500, 500, 500);

            let biosphere = GrassBiosphere::default();
            let marker = biosphere.get_marker_component();
            let builder = ServerPlanetBuilder::default();

            builder.insert_planet(&mut entity_cmd, loc, &mut structure);

            entity_cmd.insert((
                structure,
                marker,
                LoadingDistance::new(PLANET_LOAD_RADIUS, PLANET_LOAD_RADIUS + 2),
            ));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(spawn_planet.run_if(in_state(GameState::Playing)))
        .insert_resource(CachedSectors::default());
}
