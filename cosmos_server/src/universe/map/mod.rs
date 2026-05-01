//! Controls the generation and sending of map data to clients

use bevy::prelude::*;
use cosmos_core::{
    entities::{EntityId, player::Player},
    faction::{FactionId, FactionRelation, Factions},
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyMessageReceived, NettyMessageWriter},
        system_sets::NetworkingSystemsSet,
    },
    physics::location::Location,
    prelude::{Ship, Station},
    state::GameState,
    universe::{
        black_hole::BlackHole,
        map::system::{
            AsteroidDestination, BlackHoleDestination, Destination, GalaxyMap, GalaxyMapResponseMessage, PlanetDestination,
            PlayerDestination, RequestGalaxyMap, RequestSystemMap, ShipDestination, StarDestination, StationDestination, SystemMap,
            SystemMapResponseMessage,
        },
    },
};

use super::{Galaxy, SystemItem, UniverseSystems};

fn send_galaxy_map(
    mut evr_request_map: MessageReader<NettyMessageReceived<RequestGalaxyMap>>,
    mut nevw_galaxy_map: NettyMessageWriter<GalaxyMapResponseMessage>,
    q_black_hole: Query<(&Location, &BlackHole)>,
    q_galaxy: Query<&Galaxy>,
) {
    for ev in evr_request_map.read() {
        let Ok(galaxy) = q_galaxy.single() else {
            continue;
        };

        let mut g_map = GalaxyMap::default();

        for (_, star) in galaxy.iter_stars() {
            g_map.add_destination(
                star.location.sector(),
                Destination::Star(Box::new(StarDestination { star: star.star })),
            );
        }

        for (b_hole_loc, hole) in q_black_hole.iter() {
            g_map.add_destination(
                b_hole_loc.sector(),
                Destination::BlackHole(Box::new(BlackHoleDestination { black_hole: *hole })),
            );
        }

        nevw_galaxy_map.write(GalaxyMapResponseMessage { map: g_map }, ev.client_id);
    }
}

fn send_map(
    mut evr_request_map: MessageReader<NettyMessageReceived<RequestSystemMap>>,
    mut nevw_system_map: NettyMessageWriter<SystemMapResponseMessage>,

    q_players: Query<&Location, With<Player>>,
    q_stations: Query<&Location, With<Station>>,
    q_ships: Query<&Location, With<Ship>>,

    systems: Res<UniverseSystems>,

    factions: Res<Factions>,
    lobby: Res<ServerLobby>,
    q_entity: Query<(&EntityId, Option<&FactionId>)>,
) {
    for ev in evr_request_map.read() {
        let Some(player) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        let mut system_map = SystemMap::new(ev.system);

        let Some(system) = systems.system(ev.system) else {
            continue;
        };

        for (sector, danger) in system.iter_sector_danger() {
            system_map.set_danger(sector, danger);
        }

        for item in system.iter() {
            let sector = item.location.relative_sector();
            match &item.item {
                SystemItem::BlackHole(black_hole) => system_map.add_destination(
                    sector,
                    Destination::BlackHole(Box::new(BlackHoleDestination { black_hole: *black_hole })),
                ),
                SystemItem::Asteroid(_) => system_map.add_destination(sector, Destination::Asteroid(Box::new(AsteroidDestination {}))),
                SystemItem::Planet(planet) => system_map.add_destination(
                    sector,
                    Destination::Planet(Box::new(PlanetDestination {
                        location: item.location,
                        biosphere_id: planet.biosphere_id,
                    })),
                ),
                SystemItem::Star(star) => system_map.add_destination(sector, Destination::Star(Box::new(StarDestination { star: *star }))),
                SystemItem::Shop => system_map.add_destination(
                    sector,
                    Destination::Station(Box::new(StationDestination {
                        status: FactionRelation::Neutral,
                        shop_count: 1,
                    })),
                ),
                SystemItem::PirateStation(_) => system_map.add_destination(
                    sector,
                    Destination::Station(Box::new(StationDestination {
                        status: FactionRelation::Enemy,
                        shop_count: 0,
                    })),
                ),
                SystemItem::PlayerStation => system_map.add_destination(
                    sector,
                    Destination::Station(Box::new(StationDestination {
                        status: FactionRelation::Neutral,
                        shop_count: 0,
                    })),
                ),
                SystemItem::NpcStation(station) => system_map.add_destination(
                    sector,
                    Destination::Station(Box::new(StationDestination {
                        status: factions
                            .from_id(&station.faction)
                            .map(|x| {
                                let Ok((eid, fac)) = q_entity.get(player) else {
                                    return Default::default();
                                };
                                x.relation_with_entity(eid, fac.and_then(|id| factions.from_id(id)))
                            })
                            .unwrap_or(FactionRelation::Neutral),
                        shop_count: 0,
                    })),
                ),
            }
        }

        for loc in q_players.iter() {
            system_map.add_destination(
                loc.relative_sector(),
                Destination::Player(Box::new(PlayerDestination {
                    status: FactionRelation::Neutral,
                })),
            );
        }

        for loc in q_stations.iter() {
            system_map.add_destination(
                loc.relative_sector(),
                Destination::Station(Box::new(StationDestination {
                    status: FactionRelation::Neutral,
                    shop_count: 0,
                })),
            );
        }

        for loc in q_ships.iter() {
            system_map.add_destination(
                loc.relative_sector(),
                Destination::Ship(Box::new(ShipDestination {
                    status: FactionRelation::Neutral,
                })),
            );
        }

        nevw_system_map.write(
            SystemMapResponseMessage {
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
