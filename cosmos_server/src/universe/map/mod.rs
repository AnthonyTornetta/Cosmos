use bevy::{
    app::Update,
    prelude::{in_state, App, EventReader, IntoSystemConfigs, Query, Res, With},
};
use cosmos_core::{
    entities::player::Player,
    netty::{
        sync::events::server_event::{NettyEventReceived, NettyEventWriter},
        system_sets::NetworkingSystemsSet,
    },
    physics::location::Location,
    prelude::{Ship, Station},
    state::GameState,
    universe::map::system::{
        AsteroidDestination, Destination, FactionStatus, GalaxyMap, GalaxyMapResponseEvent, PlanetDestination, PlayerDestination,
        RequestGalaxyMap, RequestSystemMap, ShipDestination, StarDestination, StationDestination, SystemMap, SystemMapResponseEvent,
    },
};

use crate::universe::generation::SystemItem;

use super::{galaxy_generation::Galaxy, generation::UniverseSystems};

fn send_galaxy_map(
    mut evr_request_map: EventReader<NettyEventReceived<RequestGalaxyMap>>,
    mut nevw_galaxy_map: NettyEventWriter<GalaxyMapResponseEvent>,
    q_galaxy: Query<&Galaxy>,
) {
    for ev in evr_request_map.read() {
        let Ok(galaxy) = q_galaxy.get_single() else {
            continue;
        };

        let mut g_map = GalaxyMap::default();

        for (_, star) in galaxy.iter_stars() {
            g_map.add_destination(
                star.location.sector(),
                Destination::Star(Box::new(StarDestination { star: star.star })),
            );
        }

        nevw_galaxy_map.send(GalaxyMapResponseEvent { map: g_map }, ev.client_id);
    }
}

fn send_map(
    mut evr_request_map: EventReader<NettyEventReceived<RequestSystemMap>>,
    mut nevw_system_map: NettyEventWriter<SystemMapResponseEvent>,

    q_players: Query<&Location, With<Player>>,
    q_stations: Query<&Location, With<Station>>,
    q_ships: Query<&Location, With<Ship>>,

    systems: Res<UniverseSystems>,
) {
    for ev in evr_request_map.read() {
        let mut system_map = SystemMap::new(ev.system);

        let Some(system) = systems.system(ev.system) else {
            continue;
        };

        for item in system.iter() {
            let sector = item.location.relative_sector();
            match &item.item {
                SystemItem::Asteroid(_) => system_map.add_destination(sector, Destination::Asteroid(Box::new(AsteroidDestination {}))),
                SystemItem::Planet(planet) => system_map.add_destination(
                    sector,
                    Destination::Planet(Box::new(PlanetDestination {
                        location: item.location,
                        biosphere_id: planet.biosphere_id,
                    })),
                ),
                SystemItem::Star(star) => system_map.add_destination(sector, Destination::Star(Box::new(StarDestination { star: *star }))),
                _ => {}
            }
        }

        for loc in q_players.iter() {
            system_map.add_destination(
                loc.relative_sector(),
                Destination::Player(Box::new(PlayerDestination {
                    status: FactionStatus::Neutral,
                })),
            );
        }

        for loc in q_stations.iter() {
            system_map.add_destination(
                loc.relative_sector(),
                Destination::Station(Box::new(StationDestination {
                    status: FactionStatus::Neutral,
                    shop_count: 0,
                })),
            );
        }

        for loc in q_ships.iter() {
            system_map.add_destination(
                loc.relative_sector(),
                Destination::Ship(Box::new(ShipDestination {
                    status: FactionStatus::Neutral,
                })),
            );
        }

        nevw_system_map.send(
            SystemMapResponseEvent {
                map: system_map,
                system: ev.system,
            },
            ev.client_id,
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (send_galaxy_map, send_map)
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    );
}
