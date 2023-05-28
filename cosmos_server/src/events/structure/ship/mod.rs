//! Events for the ship

use bevy::prelude::{App, Entity, EventReader, IntoSystemConfig, OnUpdate, Query, ResMut};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{cosmos_encoder, server_unreliable_messages::ServerUnreliableMessages, NettyChannel},
    structure::ship::ship_movement::ShipMovement,
};

use crate::state::GameState;

mod core;

#[derive(Debug)]
/// This event is sent when the ship's movement is set
pub struct ShipSetMovementEvent {
    /// The entity for the ship
    pub ship: Entity,
    /// The ship's new movement
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
                cosmos_encoder::serialize(&ServerUnreliableMessages::SetMovement {
                    movement: ev.movement.clone(),
                    ship_entity: ev.ship,
                }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    core::register(app);

    app.add_event::<ShipSetMovementEvent>()
        .add_systems((monitor_set_movement_events.in_set(OnUpdate(GameState::Playing)),));
}
