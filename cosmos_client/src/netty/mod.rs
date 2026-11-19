//! Responsible for all the network information the client has

use bevy::prelude::*;
use cosmos_core::{
    netty::{
        sync::{ComponentSyncingSet, client_syncing::ClientReceiveComponents},
        system_sets::NetworkingSystemsSet,
    },
    state::GameState,
};

pub mod connect;
pub mod gameplay;
pub mod loading;
pub mod lobby;
pub mod steam;
mod sync;

fn configure_receive_order(app: &mut App) {
    // The mapping is updated immediately, but entity spawns live in each receiver's Commands.
    // Flush those spawns before gameplay packets or synced components can use their entity IDs.
    app.configure_sets(
        FixedUpdate,
        (
            ClientReceiveComponents::ClientReceiveComponents,
            gameplay::receiver::ClientReceiveGameplay,
            ComponentSyncingSet::ReceiveComponents,
        )
            .chain(),
    );
}

pub(super) fn register(app: &mut App) {
    configure_receive_order(app);
    loading::register(app);
    steam::register(app);
    connect::register(app);
    sync::register(app);

    app.configure_sets(
        FixedUpdate,
        (
            ComponentSyncingSet::PreComponentSyncing,
            ComponentSyncingSet::DoComponentSyncing,
            ComponentSyncingSet::PostComponentSyncing,
        )
            .run_if(in_state(GameState::Playing).or_else(in_state(GameState::LoadingWorld)))
            .in_set(NetworkingSystemsSet::SyncComponents)
            .chain(),
    );

    app.configure_sets(
        FixedUpdate,
        ComponentSyncingSet::ReceiveComponents
            .in_set(NetworkingSystemsSet::ReceiveMessages)
            .run_if(in_state(GameState::Playing).or_else(in_state(GameState::LoadingWorld))),
    );

    gameplay::register(app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmos_core::{
        netty::sync::mapping::NetworkMapping,
        structure::{
            Structure,
            dynamic_structure::DynamicStructure,
            planet::{Planet, biosphere::BiosphereMarker, generation::terrain_generation::PlanetTerrainSeed},
        },
    };

    #[derive(Resource)]
    struct TestServerPlanet(Entity);

    fn receive_component_entity(mut commands: Commands, mut mapping: ResMut<NetworkMapping>, server: Res<TestServerPlanet>) {
        mapping.client_from_server_or_create(&server.0, &mut commands);
    }

    fn receive_planet(mut commands: Commands, mut mapping: ResMut<NetworkMapping>, server: Res<TestServerPlanet>, entities: Query<()>) {
        let entity = mapping.client_from_server_or_create(&server.0, &mut commands);
        assert!(
            entities.contains(entity),
            "Component-replication spawns must be applied before gameplay packets use them"
        );
        commands.entity(entity).insert((
            Structure::Dynamic(DynamicStructure::new(2)),
            Planet::new(300.0),
            PlanetTerrainSeed::new(42),
            BiosphereMarker::new("test:planet"),
        ));
    }

    fn apply_planet_components(
        mut commands: Commands,
        mapping: Res<NetworkMapping>,
        server: Res<TestServerPlanet>,
        planets: Query<&Planet>,
    ) {
        let entity = mapping.client_from_server(&server.0).unwrap();
        assert!(
            planets.contains(entity),
            "The gameplay planet bundle must be applied before synced components"
        );
        commands.entity(entity).insert(Name::new("Synced planet"));
    }

    #[test]
    fn replicated_entities_are_spawned_before_gameplay_packets_and_component_application() {
        let mut app = App::new();
        app.init_resource::<NetworkMapping>();
        let server_entity = app.world_mut().spawn_empty().id();
        app.insert_resource(TestServerPlanet(server_entity));
        configure_receive_order(&mut app);
        app.add_systems(
            FixedUpdate,
            (
                receive_component_entity.in_set(ClientReceiveComponents::ClientReceiveComponents),
                receive_planet.in_set(gameplay::receiver::ClientReceiveGameplay),
                apply_planet_components.in_set(ComponentSyncingSet::ReceiveComponents),
            ),
        );

        app.world_mut().run_schedule(FixedUpdate);

        let entity = app.world().resource::<NetworkMapping>().client_from_server(&server_entity).unwrap();
        assert!(app.world().get::<Structure>(entity).is_some());
        assert!(app.world().get::<Planet>(entity).is_some());
        assert_eq!(app.world().get::<Name>(entity).unwrap().as_str(), "Synced planet");
        assert_eq!(app.world().get::<PlanetTerrainSeed>(entity).unwrap().value(), 42);
        assert_eq!(app.world().get::<BiosphereMarker>(entity).unwrap().biosphere_name(), "test:planet");
    }
}
