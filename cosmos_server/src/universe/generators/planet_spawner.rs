//! Responsible for spawning planets near stars, but for now just spawns a planet at 0, 0, 0.

use super::{
    generation::{GenerateSystemEvent, SystemGenerationSet},
    star::calculate_temperature_at,
};
use crate::{
    init::init_world::ServerSeed,
    rng::get_rng_for_sector,
    settings::ServerSettings,
    structure::planet::biosphere::BiosphereTemperatureRegistry,
    universe::{SystemItem, SystemItemPlanet, UniverseSystems},
};
use bevy::{platform::collections::HashSet, prelude::*};
use cosmos_core::{
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
    physics::location::{Location, LocationPhysicsSet, SYSTEM_SECTORS, Sector, SectorUnit, SystemCoordinate},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{
        Structure,
        coordinates::CoordinateType,
        dynamic_structure::DynamicStructure,
        planet::{PLANET_LOAD_RADIUS, Planet, biosphere::Biosphere},
    },
};
use rand::Rng;
use std::f32::consts::TAU;

#[derive(Debug, Default, Resource, Deref, DerefMut, Clone)]
struct CachedSectors(HashSet<Sector>);

fn monitor_planets_to_spawn(q_players: Query<&Location, With<Player>>, mut commands: Commands, mut systems: ResMut<UniverseSystems>) {
    let mut generated_planets = HashSet::new();

    for p_loc in q_players.iter() {
        let Some(system) = systems.system(p_loc.get_system_coordinates()) else {
            continue;
        };

        for (planet_rot, planet_loc, planet) in system
            .iter()
            .flat_map(|x| match &x.item {
                SystemItem::Planet(p) => Some((x.rotation, x.location, p)),
                _ => None,
            })
            .filter(|x| !system.is_sector_generated_for(x.1.sector(), "cosmos:planet"))
        {
            if generated_planets.contains(&planet_loc.sector()) {
                continue;
            }

            let sector_diff = (planet_loc.sector() - p_loc.sector()).abs();
            if !(sector_diff.x() <= PLANET_LOAD_RADIUS as SectorUnit
                && sector_diff.y() <= PLANET_LOAD_RADIUS as SectorUnit
                && sector_diff.z() <= PLANET_LOAD_RADIUS as SectorUnit)
            {
                continue;
            }

            let (size, loc) = (planet.size, planet_loc);

            let mut entity_cmd = commands.spawn_empty();

            let structure = Structure::Dynamic(DynamicStructure::new(size));

            info!("Creating planet entity @ {loc}");

            entity_cmd.insert((structure, planet.planet, loc, Transform::from_rotation(planet_rot)));

            generated_planets.insert(planet_loc.sector());
        }
    }

    for planet_sector in generated_planets {
        let Some(system) = systems.system_mut(SystemCoordinate::from_sector(planet_sector)) else {
            continue;
        };

        system.mark_sector_generated_for(planet_sector, "cosmos:planet");
    }
}

fn spawn_planets(
    server_seed: Res<ServerSeed>,
    mut systems: ResMut<UniverseSystems>,
    mut evr_generate_system: EventReader<GenerateSystemEvent>,

    registry: Res<BiosphereTemperatureRegistry>,
    biosphere_registry: Res<Registry<Biosphere>>,
    settings: Res<ServerSettings>,
) {
    if !settings.spawn_planets {
        return;
    }

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

        let n_planets: usize = rng.random_range(0..30);

        for _ in 0..n_planets {
            let sector = Sector::new(
                rng.random_range(0..(SYSTEM_SECTORS as SectorUnit)),
                rng.random_range(0..(SYSTEM_SECTORS as SectorUnit)),
                rng.random_range(0..(SYSTEM_SECTORS as SectorUnit)),
            ) + star_loc.get_system_coordinates().negative_most_sector();

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
                    2_f32.powi(rng.random_range(7..=9)) as CoordinateType
                };

                let biospheres = registry.get_biospheres_for(temperature);

                if biospheres.is_empty() {
                    warn!(
                        "No biosphere for temperature {} @ sector {sector} - this planet will not be generated!",
                        temperature
                    );
                }

                let biosphere_name = biospheres[rng.random_range(0..biospheres.len())];
                let biosphere_id = biosphere_registry
                    .from_id(biosphere_name)
                    .unwrap_or_else(|| panic!("Missing biosphere {biosphere_name}"))
                    .id();

                let angle = rng.random::<f32>() % TAU;
                let axis = Dir3::new(Vec3::new(rng.random(), rng.random(), rng.random()).normalize_or_zero()).unwrap_or(Dir3::Y);

                system.add_item(
                    location,
                    Quat::from_axis_angle(*axis, angle),
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

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            spawn_planets.in_set(SystemGenerationSet::Planet),
            monitor_planets_to_spawn.in_set(NetworkingSystemsSet::Between),
        )
            .chain()
            .before(LocationPhysicsSet::DoPhysics)
            .run_if(in_state(GameState::Playing)),
    )
    .insert_resource(CachedSectors::default());
}
