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
    prelude::{Asteroid, Planet, Ship, Station},
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    structure::planet::biosphere::{Biosphere, BiosphereMarker},
    universe::{
        map::system::{
            AsteroidDestination, Destination, FactionStatus, PlanetDestination, PlayerDestination, RequestSystemMap, ShipDestination,
            StarDestination, StationDestination, SystemMap, SystemMapResponseEvent,
        },
        star::Star,
    },
};

fn send_map(
    mut evr_request_map: EventReader<NettyEventReceived<RequestSystemMap>>,
    mut nevw_system_map: NettyEventWriter<SystemMapResponseEvent>,

    biospheres: Res<Registry<Biosphere>>,
    q_planets: Query<(&Location, &BiosphereMarker), With<Planet>>,
    q_star: Query<(&Location, &Star)>,
    q_players: Query<&Location, With<Player>>,
    q_stations: Query<&Location, With<Station>>,
    q_asteroid: Query<&Location, With<Asteroid>>,
    q_ships: Query<&Location, With<Ship>>,
) {
    for ev in evr_request_map.read() {
        println!("Got: {ev:?} -- sending response!");

        let mut system_map = SystemMap::default();

        for (loc, biosphere_marker) in q_planets.iter() {
            let biosphere = biospheres
                .from_id(biosphere_marker.biosphere_name())
                .expect("Failed to get biosphere from unlocalized id!");

            system_map.add_destination(
                loc.relative_sector(),
                Destination::Planet(Box::new(PlanetDestination {
                    location: *loc,
                    biosphere_id: biosphere.id(),
                })),
            );
        }

        for (loc, star) in q_star.iter() {
            system_map.add_destination(loc.relative_sector(), Destination::Star(Box::new(StarDestination { star: *star })));
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

        for loc in q_asteroid.iter() {
            system_map.add_destination(loc.relative_sector(), Destination::Asteroid(Box::new(AsteroidDestination {})));
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
        send_map.in_set(NetworkingSystemsSet::Between).run_if(in_state(GameState::Playing)),
    );
}
