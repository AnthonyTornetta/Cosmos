use bevy::prelude::{App, Entity, EventReader, IntoSystemConfig, OnUpdate, Query, ResMut};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    events::structure::change_pilot_event::ChangePilotEvent,
    netty::{
        network_encoder, server_reliable_messages::ServerReliableMessages,
        server_unreliable_messages::ServerUnreliableMessages, NettyChannel,
    },
    structure::ship::ship_movement::ShipMovement,
};

use crate::state::GameState;

pub mod core;

pub struct ShipSetMovementEvent {
    pub ship: Entity,
    pub movement: ShipMovement,
}

fn monitor_set_movement_events(
    mut query: Query<&mut ShipMovement>,
    mut event_reader: EventReader<ShipSetMovementEvent>,
    mut server: ResMut<RenetServer>,
) {
    for ev in event_reader.iter() {
        if let Ok(mut current_movement) = query.get_mut(ev.ship) {
            current_movement.set(&ev.movement);

            server.broadcast_message(
                NettyChannel::Unreliable.id(),
                network_encoder::serialize(&ServerUnreliableMessages::SetMovement {
                    movement: ev.movement.clone(),
                    ship_entity: ev.ship,
                }),
            );
        }
    }
}

fn monitor_pilot_changes(
    mut event_reader: EventReader<ChangePilotEvent>,
    mut server: ResMut<RenetServer>,
) {
    for ev in event_reader.iter() {
        server.broadcast_message(
            NettyChannel::Reliable.id(),
            network_encoder::serialize(&ServerReliableMessages::PilotChange {
                structure_entity: ev.structure_entity,
                pilot_entity: ev.pilot_entity,
            }),
        );
    }
}

pub(crate) fn register(app: &mut App) {
    core::register(app);

    app.add_event::<ShipSetMovementEvent>().add_systems((
        monitor_set_movement_events.in_set(OnUpdate(GameState::Playing)),
        monitor_pilot_changes.in_set(OnUpdate(GameState::Playing)),
    ));
}
