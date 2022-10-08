use bevy::prelude::{App, Entity, EventReader, Query, ResMut, SystemSet};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    events::structure::change_pilot_event::ChangePilotEvent,
    netty::{
        server_reliable_messages::ServerReliableMessages,
        server_unreliable_messages::ServerUnreliableMessages, NettyChannel,
    },
    structure::ship::ship_movement::ShipMovement,
};

use crate::state::GameState;

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
                bincode::serialize(&ServerUnreliableMessages::SetMovement {
                    movement: ev.movement.clone(),
                    ship_entity: ev.ship,
                })
                .unwrap(),
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
            bincode::serialize(&ServerReliableMessages::PilotChange {
                structure_entity: ev.structure_entity,
                pilot_entity: ev.pilot_entity,
            })
            .unwrap(),
        );
    }
}

pub fn register(app: &mut App) {
    app.add_event::<ShipSetMovementEvent>().add_system_set(
        SystemSet::on_update(GameState::Playing)
            .with_system(monitor_set_movement_events)
            .with_system(monitor_pilot_changes),
    );
}
