//! Events for the ship

use bevy::prelude::*;
use bevy_rapier3d::dynamics::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::block_events::BlockEventsSet,
    events::structure::change_pilot_event::ChangePilotEvent,
    netty::{
        NettyChannelServer, cosmos_encoder, server_reliable_messages::ServerReliableMessages,
        server_unreliable_messages::ServerUnreliableMessages, system_sets::NetworkingSystemsSet,
    },
    physics::location::Location,
    prelude::Ship,
    state::GameState,
    structure::{
        Structure, StructureTypeSet, coordinates::ChunkCoordinate, full_structure::FullStructure, loading::StructureLoadingSet,
        ship::ship_movement::ShipMovement,
    },
};

use crate::ai::AiControlled;

use super::loading::ShipNeedsCreated;

#[derive(Debug, Event)]
/// This event is sent when the ship's movement is set
pub struct ShipSetMovementEvent {
    /// The entity for the ship
    pub ship: Entity,
    /// The ship's new movement
    pub movement: ShipMovement,
}

fn monitor_set_movement_events(
    mut query: Query<&mut ShipMovement, Without<AiControlled>>, // don't sync AI controlled movements to not give players that knowledge
    mut event_reader: EventReader<ShipSetMovementEvent>,
    mut server: ResMut<RenetServer>,
) {
    for ev in event_reader.read() {
        if let Ok(mut current_movement) = query.get_mut(ev.ship) {
            *current_movement = ev.movement;

            server.broadcast_message(
                NettyChannelServer::Unreliable,
                cosmos_encoder::serialize(&ServerUnreliableMessages::SetMovement {
                    movement: ev.movement,
                    ship_entity: ev.ship,
                }),
            );
        }
    }
}

fn monitor_pilot_changes(mut event_reader: EventReader<ChangePilotEvent>, mut server: ResMut<RenetServer>) {
    for ev in event_reader.read() {
        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::PilotChange {
                structure_entity: ev.structure_entity,
                pilot_entity: ev.pilot_entity,
            }),
        );
    }
}

/// This event is done when a ship is being created
#[derive(Debug, Event)]
pub struct CreateShipEvent {
    /// The entity (likely a player) that created this ship.
    pub creator: Entity,
    /// Starting location of the ship
    pub ship_location: Location,
    /// The rotation of the ship
    pub rotation: Quat,
}

pub(crate) fn create_ship_event_reader(mut event_reader: EventReader<CreateShipEvent>, mut commands: Commands) {
    for ev in event_reader.read() {
        info!("Creating ship!!");

        let mut entity = commands.spawn_empty();

        let structure = Structure::Full(FullStructure::new(ChunkCoordinate::new(10, 10, 10)));

        entity.insert((
            structure,
            ev.ship_location,
            Velocity::default(),
            Ship,
            ShipNeedsCreated,
            Transform::from_rotation(ev.rotation),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<ShipSetMovementEvent>().add_systems(
        FixedUpdate,
        (monitor_pilot_changes, monitor_set_movement_events)
            .after(BlockEventsSet::PostProcessEvents)
            .in_set(StructureTypeSet::Ship)
            // TODO: this in_set makes no sense - check this
            .in_set(NetworkingSystemsSet::SyncComponents)
            .run_if(in_state(GameState::Playing)),
    );

    app.add_event::<CreateShipEvent>().add_systems(
        FixedUpdate,
        create_ship_event_reader
            .in_set(StructureLoadingSet::LoadStructure)
            .run_if(in_state(GameState::Playing)),
    );
}
