//! Responsible for spawning planets near stars, but for now just spawns a planet at 0, 0, 0.

use std::time::Duration;

use bevy::{
    log::error,
    prelude::{in_state, App, Commands, Deref, DerefMut, EventReader, IntoSystemConfigs, Query, Res, ResMut, Resource, Update, Vec3, With},
    time::common_conditions::on_timer,
    utils::HashSet,
};
use cosmos_core::{
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
    physics::location::{Location, Sector, SectorUnit, SystemCoordinate, SystemUnit, SECTOR_DIMENSIONS, SYSTEM_SECTORS},
    state::GameState,
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
    init::init_world::ServerSeed,
    persistence::is_sector_generated,
    rng::get_rng_for_sector,
    settings::ServerSettings,
    structure::asteroid::server_asteroid_builder::ServerAsteroidBuilder,
    universe::{generation::SystemItem, star::calculate_temperature_at},
};

use super::generation::{GenerateSystemEvent, GeneratedItem, SystemItemAsteroid, UniverseSystems};

#[derive(Default, Resource, Deref, DerefMut)]
struct CachedSectors(HashSet<Sector>);

fn spawn_asteroids(
    mut evr_create_system: EventReader<GenerateSystemEvent>,
    // query: Query<&Location, With<Asteroid>>,
    // players: Query<&Location, With<Player>>,
    server_seed: Res<ServerSeed>,
    // mut cache: ResMut<CachedSectors>,
    mut commands: Commands,
    mut systems: ResMut<UniverseSystems>,
    // q_stars: Query<(&Location, &Star)>,
    settings: Res<ServerSettings>,
) {
    if !settings.spawn_asteroids {
        return;
    }

    for ev in evr_create_system.read() {
        let Some(system) = systems.system_mut(ev.system) else {
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
            continue;
        };

        let star_sector = star_loc.sector();
        let mut rng = get_rng_for_sector(&server_seed, &star_sector);

        let n_asteroid_sectors: usize = rng.gen_range(0..20);

        for _ in 0..n_asteroid_sectors {
            let sector = Sector::new(
                rng.gen_range(0..(SYSTEM_SECTORS as SectorUnit)),
                rng.gen_range(0..(SYSTEM_SECTORS as SectorUnit)),
                rng.gen_range(0..(SYSTEM_SECTORS as SectorUnit)),
            ) + star_loc.get_system_coordinates().negative_most_sector();

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

fn generate_asteroids(mut commands: Commands, mut systems: ResMut<UniverseSystems>) {
    let mut sectors_to_mark = HashSet::new();

    for (_, universe_system) in systems.iter() {
        for (asteroid_loc, asteroid) in universe_system.iter().flat_map(|x| match &x.item {
            SystemItem::Asteroid(a) => Some((x.location, a)),
            _ => None,
        }) {
            if universe_system.is_sector_generated_for(asteroid_loc.sector(), "cosmos:asteroid") {
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
        spawn_asteroids
            .in_set(NetworkingSystemsSet::Between)
            .run_if(on_timer(Duration::from_secs(1)))
            .run_if(in_state(GameState::Playing)),
    )
    .insert_resource(CachedSectors::default());
}
