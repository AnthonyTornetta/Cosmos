//! Responsible for spawning planets near stars, but for now just spawns a planet at 0, 0, 0.

use std::f32::consts::PI;

use bevy::{
    log::{error, warn},
    math::Quat,
    prelude::{in_state, App, Commands, Deref, DerefMut, EventReader, IntoSystemConfigs, Query, Res, ResMut, Resource, Update, Vec3, With},
    utils::HashSet,
};
use cosmos_core::{
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
    physics::location::{Location, Sector, SectorUnit, SystemCoordinate, SystemUnit, SECTOR_DIMENSIONS, SYSTEM_SECTORS},
    state::GameState,
    structure::{
        asteroid::{asteroid_builder::TAsteroidBuilder, loading::AsteroidNeedsCreated, ASTEROID_LOAD_RADIUS},
        coordinates::ChunkCoordinate,
        full_structure::FullStructure,
        Structure,
    },
    utils::quat_math::random_quat,
};
use rand::Rng;

use crate::{
    init::init_world::ServerSeed,
    rng::get_rng_for_sector,
    settings::ServerSettings,
    structure::asteroid::server_asteroid_builder::ServerAsteroidBuilder,
    universe::{generation::SystemItem, star::calculate_temperature_at},
};

use super::generation::{GenerateSystemEvent, SystemGenerationSet, SystemItemAsteroid, UniverseSystems};

#[derive(Default, Resource, Deref, DerefMut)]
struct CachedSectors(HashSet<Sector>);

fn spawn_asteroids(
    mut evr_create_system: EventReader<GenerateSystemEvent>,
    server_seed: Res<ServerSeed>,
    mut systems: ResMut<UniverseSystems>,
    settings: Res<ServerSettings>,
) {
    if !settings.spawn_asteroids {
        return;
    }

    for ev in evr_create_system.read() {
        let Some(system) = systems.system_mut(ev.system) else {
            warn!("Missing system @ {}", ev.system);
            continue;
        };

        let star = system
            .iter()
            .flat_map(|x| match x.item {
                SystemItem::Star(star) => Some((x.location, star)),
                _ => None,
            })
            .next();

        let Some((star_loc, star)) = star else {
            warn!("Missing star in system {}", ev.system);
            continue;
        };

        let star_sector = star_loc.sector();
        let mut rng = get_rng_for_sector(&server_seed, &star_sector);

        // Favors lower numbers
        let n_asteroid_rings: usize = (1.0 + 5.0 * (1.0 - (1.0 - rng.gen::<f32>()).sqrt())) as usize;

        for _ in 0..n_asteroid_rings {
            let ring_diameter = rng.gen_range(10..=90);
            let circum = ring_diameter as f32 * PI;
            let n_iterations = (circum * rng.gen_range(1..=6) as f32) as SectorUnit;
            let asteroid_axis = random_quat(&mut rng);

            for i in 0..n_iterations {
                let angle = (i as f32 * PI * 2.0) / (n_iterations as f32);
                let coordinate = asteroid_axis * Quat::from_axis_angle(Vec3::Y, angle) * (Vec3::NEG_Z * ring_diameter as f32 / 2.0);

                let sector = Sector::new(
                    coordinate.x.round() as SectorUnit,
                    coordinate.y.round() as SectorUnit,
                    coordinate.z.round() as SectorUnit,
                ) + star_loc.get_system_coordinates().negative_most_sector()
                    + Sector::splat(SYSTEM_SECTORS as SectorUnit / 2);

                // Don't generate asteroids if something is already here
                if system.items_at(sector).next().is_some() {
                    continue;
                }

                let n_asteroids = (6.0 * (1.0 - (1.0 - rng.gen::<f32>()).sqrt())) as usize;

                let multiplier = SECTOR_DIMENSIONS;
                let adder = -SECTOR_DIMENSIONS / 2.0;

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

                    let Some(temperature) = calculate_temperature_at([(star_loc, star)].iter(), &loc) else {
                        continue;
                    };

                    system.add_item(loc, SystemItem::Asteroid(SystemItemAsteroid { size, temperature }));
                }
            }
        }
    }
}

fn generate_asteroids(mut commands: Commands, q_players: Query<&Location, With<Player>>, mut systems: ResMut<UniverseSystems>) {
    let mut sectors_to_mark = HashSet::new();

    for (_, universe_system) in systems.iter() {
        for (asteroid_loc, asteroid) in universe_system.iter().flat_map(|x| match &x.item {
            SystemItem::Asteroid(a) => Some((x.location, a)),
            _ => None,
        }) {
            if universe_system.is_sector_generated_for(asteroid_loc.sector(), "cosmos:asteroid") {
                continue;
            }

            if !q_players
                .iter()
                .any(|loc| (loc.sector() - asteroid_loc.sector()).abs().max_element() <= ASTEROID_LOAD_RADIUS as SystemUnit)
            {
                continue;
            }

            sectors_to_mark.insert(asteroid_loc.sector());

            let mut structure = Structure::Full(FullStructure::new(ChunkCoordinate::new(
                asteroid.size,
                asteroid.size,
                asteroid.size,
            )));
            let builder = ServerAsteroidBuilder::default();
            let mut entity_cmd = commands.spawn_empty();

            builder.insert_asteroid(&mut entity_cmd, asteroid_loc, &mut structure, asteroid.temperature);

            entity_cmd.insert((structure, AsteroidNeedsCreated));
        }
    }

    for sector in sectors_to_mark {
        let Some(system) = systems.system_mut(SystemCoordinate::from_sector(sector)) else {
            error!("Unloaded system but tried to load asteroids in it???");
            continue;
        };

        system.mark_sector_generated_for(sector, "cosmos:asteroid");
    }
}
// let mut to_check_sectors = HashSet::new();
//
// for l in players.iter() {
//     for dsz in -(ASTEROID_LOAD_RADIUS as SystemUnit)..=ASTEROID_LOAD_RADIUS as SystemUnit {
//         for dsy in -(ASTEROID_LOAD_RADIUS as SystemUnit)..=ASTEROID_LOAD_RADIUS as SystemUnit {
//             for dsx in -(ASTEROID_LOAD_RADIUS as SystemUnit)..=ASTEROID_LOAD_RADIUS as SystemUnit {
//                 let sector = l.sector() + Sector::new(dsx, dsy, dsz);
//                 to_check_sectors.insert(sector);
//             }
//         }
//     }
// }
//
// let mut dead_sectors = HashSet::new();
//
// // Clear out unloaded sectors from the cache
// for sector in cache.iter() {
//     if !to_check_sectors.contains(sector) {
//         dead_sectors.insert(*sector);
//     }
// }
//
//     for dead_sector in dead_sectors {
//         cache.remove(&dead_sector);
//     }
//
//     let mut sectors = HashSet::new();
//
//     for sector in to_check_sectors {
//         if !cache.contains(&sector) {
//             sectors.insert(sector);
//         }
//     }
//
//     for loc in query.iter() {
//         let sector = loc.sector();
//         cache.insert(sector);
//         sectors.remove(&sector);
//     }
//
//     for sector in sectors {
//         cache.insert(sector);
//
//         if is_sector_generated(sector) || is_planet_in_sector(&sector, &server_seed) {
//             // This sector has already been loaded, don't regenerate stuff
//             continue;
//         }
//
//         let mut rng = get_rng_for_sector(&server_seed, &sector);
//
//         if rng.gen_range(0..1000) < 100 {
//             // Biased towards lower amounts
//             let n_asteroids = (6.0 * (1.0 - (1.0 - rng.gen::<f32>()).sqrt())) as usize;
//
//             let multiplier = SECTOR_DIMENSIONS;
//             let adder = -SECTOR_DIMENSIONS / 2.0;
//
//             let stars = q_stars.iter().map(|(x, y)| (*x, *y)).collect::<Vec<(Location, Star)>>();
//
//             for _ in 0..n_asteroids {
//                 let size = rng.gen_range(4..=8);
//
//                 let loc = Location::new(
//                     Vec3::new(
//                         rng.gen::<f32>() * multiplier + adder,
//                         rng.gen::<f32>() * multiplier + adder,
//                         rng.gen::<f32>() * multiplier + adder,
//                     ),
//                     sector,
//                 );
//
//                 if let Some(temperature) = calculate_temperature_at(stars.iter(), &loc) {
//                     let mut structure = Structure::Full(FullStructure::new(ChunkCoordinate::new(size, size, size)));
//                     let builder = ServerAsteroidBuilder::default();
//                     let mut entity_cmd = commands.spawn_empty();
//
//                     builder.insert_asteroid(&mut entity_cmd, loc, &mut structure, temperature);
//
//                     entity_cmd.insert((structure, AsteroidNeedsCreated));
//                 }
//             }
//         }
//     }
// }

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (spawn_asteroids.in_set(SystemGenerationSet::Asteroid), generate_asteroids)
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    )
    .insert_resource(CachedSectors::default());
}
