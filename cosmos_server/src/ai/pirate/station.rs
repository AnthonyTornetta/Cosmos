//! Controls pirate stations

use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use cosmos_core::{
    netty::sync::IdentifiableComponent,
    physics::location::{Location, SystemCoordinate},
    structure::shared::MeltingDown,
    time::UniverseTimestamp,
    utils::random::random_range,
};
use serde::{Deserialize, Serialize};

use crate::{
    persistence::make_persistent::{DefaultPersistentComponent, make_persistent},
    universe::{
        UniverseSystems,
        spawners::pirate::{MAX_PIRATE_DIFFICULTY, PirateNeedsSpawned},
    },
};

#[derive(Component, Serialize, Deserialize, Debug)]
/// A station that is run by pirates
pub struct PirateStation;

impl IdentifiableComponent for PirateStation {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:pirate_station"
    }
}

impl DefaultPersistentComponent for PirateStation {}

#[derive(Component, Serialize, Deserialize, Reflect, Clone, Copy, Debug, Default)]
struct PirateStationPirateShipSpawner {
    last_spawned: UniverseTimestamp,
}

impl IdentifiableComponent for PirateStationPirateShipSpawner {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:pirate_station_pirate_spawner"
    }
}

impl DefaultPersistentComponent for PirateStationPirateShipSpawner {}

const MAX_PIRATE_SPAWNS: u32 = 7;

fn add_spawner_on_new_station(
    mut commands: Commands,
    q_station: Query<Entity, (Added<PirateStation>, Without<PirateStationPirateShipSpawner>)>,
) {
    for e in q_station.iter() {
        commands.entity(e).insert(PirateStationPirateShipSpawner::default());
    }
}

const SECS_PER_PIRATE: u64 = 200;

fn spawn_pirates_for_station(
    mut commands: Commands,
    mut q_needs_pirates_spawned: Query<(&Location, &mut PirateStationPirateShipSpawner), Without<MeltingDown>>,
    timestamp: Res<UniverseTimestamp>,
    universe_systems: Res<UniverseSystems>,
) {
    const MIN_SPAWN_RADIUS: f32 = 400.0;
    const MAX_SPAWN_RADIUS: f32 = 1000.0;
    for (loc, mut spawner) in q_needs_pirates_spawned.iter_mut() {
        if let Some(d) = (*timestamp - spawner.last_spawned).map(|x| x.as_secs()) {
            if d < SECS_PER_PIRATE {
                continue;
            }

            spawner.last_spawned = *timestamp;

            let n_pirates = (d / SECS_PER_PIRATE).min(MAX_PIRATE_SPAWNS as u64);

            for _ in 0..n_pirates {
                let sys_coord = SystemCoordinate::from_sector(loc.sector());
                let Some(sys) = universe_systems.system(sys_coord) else {
                    continue;
                };

                let danger = sys
                    .sector_danger(loc.sector() - sys_coord.negative_most_sector())
                    .bounded()
                    .max(0.2)
                    * (MAX_PIRATE_DIFFICULTY + 1) as f32;
                let difficulty = (danger.round() as u32).max(1) - 1;

                let spawn_offset = Vec3::new(
                    random_range(MIN_SPAWN_RADIUS, MAX_SPAWN_RADIUS),
                    random_range(MIN_SPAWN_RADIUS, MAX_SPAWN_RADIUS),
                    random_range(MIN_SPAWN_RADIUS, MAX_SPAWN_RADIUS),
                );

                commands.spawn((
                    Name::new("Loading pirate ship"),
                    PirateNeedsSpawned {
                        difficulty,
                        location: *loc + spawn_offset,
                        heading_towards: *loc + spawn_offset * 3.0,
                    },
                ));
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<PirateStation>(app);
    make_persistent::<PirateStationPirateShipSpawner>(app);

    app.add_systems(
        Update,
        (
            add_spawner_on_new_station,
            spawn_pirates_for_station.run_if(on_timer(Duration::from_secs(1))).chain(),
        ),
    )
    .register_type::<PirateStationPirateShipSpawner>();
}
