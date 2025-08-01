use bevy::prelude::*;
use cosmos_core::{
    physics::location::{Location, SYSTEM_SECTORS, Sector},
    state::GameState,
    utils::quat_math::random_quat,
};
use rand::{Rng, seq::IteratorRandom};

use crate::{
    init::init_world::ServerSeed,
    rng::get_rng_for_sector,
    universe::{SectorDanger, SystemItem, SystemItemPirateStation, UniverseSystems},
};

use super::generation::{GenerateSystemEvent, SystemGenerationSet};

const PIRATE_STATION_MIN_DANGER: SectorDanger = SectorDanger::MIDDLE;

fn generate_pirate_stations(
    mut evr_generate_system: EventReader<GenerateSystemEvent>,
    server_seed: Res<ServerSeed>,
    mut systems: ResMut<UniverseSystems>,
) {
    for ev in evr_generate_system.read() {
        let Some(system) = systems.system_mut(ev.system) else {
            continue;
        };

        const SECTOR_SEED_OFFSET: Sector = Sector::new(120, 151, 23);

        let mut rng = get_rng_for_sector(&server_seed, &(ev.system.negative_most_sector() + SECTOR_SEED_OFFSET));

        let n_stations = rng.random_range(20..=50);

        let mut done_zones = vec![];

        let mut n_asteroid_stations = rng.random_range(10..=n_stations / 2);

        for _ in 0..n_stations {
            let mut pirate_station_sector = if n_asteroid_stations != 0 {
                system
                    .iter()
                    .filter(|maybe_asteroid| matches!(maybe_asteroid.item, SystemItem::Asteroid(_)))
                    .map(|asteroid| asteroid.location.sector)
                    .choose(&mut rng)
                    .unwrap_or_else(|| {
                        Sector::new(
                            rng.random_range(0..SYSTEM_SECTORS as i64),
                            rng.random_range(0..SYSTEM_SECTORS as i64),
                            rng.random_range(0..SYSTEM_SECTORS as i64),
                        )
                    })
            } else {
                system.coordinate().negative_most_sector()
                    + Sector::new(
                        rng.random_range(0..SYSTEM_SECTORS as i64),
                        rng.random_range(0..SYSTEM_SECTORS as i64),
                        rng.random_range(0..SYSTEM_SECTORS as i64),
                    )
            };

            pirate_station_sector = pirate_station_sector + ev.system.negative_most_sector();

            if done_zones.contains(&pirate_station_sector) {
                if n_asteroid_stations != 0 {
                    n_asteroid_stations -= 1;
                }

                continue;
            }

            let sector_danger = system.compute_sector_danger(pirate_station_sector);

            if sector_danger < PIRATE_STATION_MIN_DANGER {
                if n_asteroid_stations != 0 {
                    n_asteroid_stations -= 1;
                }

                // Don't generate too close to safe things
                continue;
            }

            done_zones.push(pirate_station_sector);

            system.add_item(
                Location::new(Vec3::ZERO, pirate_station_sector),
                random_quat(&mut rng),
                SystemItem::PirateStation(SystemItemPirateStation {
                    build_type: "default".into(),
                }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        generate_pirate_stations
            .in_set(SystemGenerationSet::PirateStation)
            .run_if(in_state(GameState::Playing)),
    );
}
