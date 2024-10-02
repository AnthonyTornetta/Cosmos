//! Responsible for spawning planets near stars, but for now just spawns a planet at 0, 0, 0.

use super::{
    generation::{GenerateSystemEvent, SystemGenerationSet, SystemItem, SystemItemPlanet, UniverseSystems},
    star::calculate_temperature_at,
};
use crate::{
    init::init_world::ServerSeed,
    rng::get_rng_for_sector,
    structure::planet::{biosphere::BiosphereTemperatureRegistry, server_planet_builder::ServerPlanetBuilder},
};
use bevy::{
    log::warn,
    math::{Dir3, Quat},
    prelude::{
        in_state, App, Commands, Component, Deref, DerefMut, EventReader, IntoSystemConfigs, Query, Res, ResMut, Resource, Update, Vec3,
        With,
    },
    tasks::Task,
    utils::HashSet,
};
use cosmos_core::{
    ecs::bundles::BundleStartingRotation,
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
    physics::location::{Location, Sector, SectorUnit},
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    structure::{
        coordinates::CoordinateType,
        dynamic_structure::DynamicStructure,
        planet::{biosphere::Biosphere, planet_builder::TPlanetBuilder, Planet, PLANET_LOAD_RADIUS},
        Structure,
    },
};
use rand::Rng;
use std::f32::consts::TAU;

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
    q_planets: Query<&Location, With<Planet>>,
    q_players: Query<&Location, With<Player>>,
    mut commands: Commands,
    server_seed: Res<ServerSeed>,
    systems: Res<UniverseSystems>,
) {
    let spawned_planets = q_planets.iter().map(|x| x.sector()).collect::<HashSet<Sector>>();

    for p_loc in q_players.iter() {
        let Some(system) = systems.system(p_loc.get_system_coordinates()) else {
            continue;
        };

        for (planet_loc, planet) in system
            .iter()
            .flat_map(|x| match &x.item {
                SystemItem::Planet(p) => Some((x.location, p)),
                _ => None,
            })
            .filter(|x| !spawned_planets.contains(&x.0.sector()))
        {
            let sector_diff = (planet_loc.sector() - p_loc.sector()).abs();
            if !(sector_diff.x() <= PLANET_LOAD_RADIUS as SectorUnit
                && sector_diff.y() <= PLANET_LOAD_RADIUS as SectorUnit
                && sector_diff.z() <= PLANET_LOAD_RADIUS as SectorUnit)
            {
                continue;
            }

            let (size, loc) = (planet.size, planet_loc);

            let mut entity_cmd = commands.spawn_empty();

            let mut structure = Structure::Dynamic(DynamicStructure::new(size));

            let builder = ServerPlanetBuilder::default();

            builder.insert_planet(&mut entity_cmd, loc, &mut structure, planet.planet);

            let mut rng = get_rng_for_sector(&server_seed, &loc.sector);

            let angle = rng.gen::<f32>() % TAU;
            let axis = Dir3::new(Vec3::new(rng.gen(), rng.gen(), rng.gen()).normalize_or_zero()).unwrap_or(Dir3::Y);

            entity_cmd.insert((structure, BundleStartingRotation(Quat::from_axis_angle(*axis, angle))));
        }
    }
}

fn spawn_planets(
    server_seed: Res<ServerSeed>,
    mut systems: ResMut<UniverseSystems>,
    mut evr_generate_system: EventReader<GenerateSystemEvent>,

    registry: Res<BiosphereTemperatureRegistry>,
    biosphere_registry: Res<Registry<Biosphere>>,
) {
    for ev in evr_generate_system.read() {
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

        let n_planets: usize = rng.gen_range(0..20);

        for _ in 0..n_planets {
            let sector = Sector::new(rng.gen_range(0..100), rng.gen_range(0..100), rng.gen_range(0..100))
                + star_loc.get_system_coordinates().negative_most_sector();

            let location = Location::new(Vec3::ZERO, sector);

            // Don't generate a planet if something is already here
            if system.items_at(location.sector()).next().is_some() {
                continue;
            }

            if let Some(temperature) = calculate_temperature_at([(star_loc, star)].iter(), &location) {
                let is_origin = star_sector.x() == 25 && star_sector.y() == 25 && star_sector.z() == 25;

                let size = if is_origin {
                    64
                } else {
                    2_f32.powi(rng.gen_range(7..=9)) as CoordinateType
                };

                let biospheres = registry.get_biospheres_for(temperature);

                if biospheres.is_empty() {
                    warn!(
                        "No biosphere for temperature {} @ sector {sector} - this planet will not be generated!",
                        temperature
                    );
                }

                let biosphere_name = biospheres[rng.gen_range(0..biospheres.len())];
                let biosphere_id = biosphere_registry
                    .from_id(biosphere_name)
                    .unwrap_or_else(|| panic!("Missing biosphere {biosphere_name}"))
                    .id();

                system.add_item(
                    location,
                    SystemItem::Planet(SystemItemPlanet {
                        size,
                        planet: Planet::new(temperature),
                        biosphere_id,
                    }),
                );
            }
        }
    }
}
//
// fn spawn_planet(
//     q_planet_locations: Query<&Location, With<Planet>>,
//     q_player_locations: Query<&Location, With<Player>>,
//     server_seed: Res<ServerSeed>,
//     mut commands: Commands,
//     stars: Query<(&Location, &Star), With<Star>>,
//     cache: Res<CachedSectors>,
//     is_already_generating: Query<(), With<PlanetSpawnerAsyncTask>>,
//     server_settings: Res<ServerSettings>,
// ) {
//     if !server_settings.spawn_planets {
//         return;
//     }
//
//     if !is_already_generating.is_empty() {
//         // an async task is already running, don't make another one
//         return;
//     }
//
//     if q_player_locations.is_empty() {
//         // Don't bother if there are no players
//         return;
//     }
//
//     let thread_pool = AsyncComputeTaskPool::get();
//
//     let locs = q_player_locations.iter().copied().collect::<Vec<Location>>();
//
//     let mut cache = cache.clone();
//
//     q_planet_locations.iter().for_each(|l| {
//         cache.insert(l.sector());
//     });
//
//     let server_seed = *server_seed;
//     let stars = stars.iter().map(|(x, y)| (*x, *y)).collect::<Vec<(Location, Star)>>();
//
//     let task = thread_pool.spawn(async move {
//         let mut to_check_sectors = HashSet::new();
//
//         for l in locs {
//             for dsz in -(PLANET_LOAD_RADIUS as SystemUnit)..=(PLANET_LOAD_RADIUS as SystemUnit) {
//                 for dsy in -(PLANET_LOAD_RADIUS as SystemUnit)..=(PLANET_LOAD_RADIUS as SystemUnit) {
//                     for dsx in -(PLANET_LOAD_RADIUS as SystemUnit)..=(PLANET_LOAD_RADIUS as SystemUnit) {
//                         let sector = l.sector() + Sector::new(dsx, dsy, dsz);
//                         if !cache.contains(&sector) {
//                             to_check_sectors.insert(sector);
//                         }
//                     }
//                 }
//             }
//         }
//
//         let mut made_stars = vec![];
//
//         for sector in to_check_sectors {
//             cache.insert(sector);
//
//             if is_sector_generated(sector) {
//                 // This sector has already been loaded, don't regenerate stuff
//                 continue;
//             }
//
//             let mut rng = get_rng_for_sector(&server_seed, &sector);
//
//             let is_origin = sector.x() == 25 && sector.y() == 25 && sector.z() == 25;
//
//             if is_origin || rng.gen_range(0..1000) == 9 {
//                 let location = Location::new(Vec3::ZERO, sector);
//
//                 if let Some(temperature) = calculate_temperature_at(stars.iter(), &location) {
//                     let size = if is_origin {
//                         64
//                     } else {
//                         2_f32.powi(rng.gen_range(7..=9)) as CoordinateType
//                     };
//
//                     made_stars.push(PlanetToSpawn {
//                         size,
//                         temperature,
//                         location,
//                     });
//                 }
//             }
//         }
//
//         (cache, made_stars)
//     });
//
//     commands.spawn((Name::new("Planet spawner async task"), PlanetSpawnerAsyncTask(task)));
// }
//
pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (monitor_planets_to_spawn.in_set(SystemGenerationSet::Planet), spawn_planets)
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    )
    .insert_resource(CachedSectors::default());
}
