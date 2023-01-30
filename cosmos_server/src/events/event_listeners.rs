use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{
        netty_rigidbody::NettyRigidBody, server_reliable_messages::ServerReliableMessages,
        NettyChannel,
    },
    structure::{events::StructureCreated, planet::Planet, ship::Ship, Structure},
};

fn on_structure_created(
    mut server: ResMut<RenetServer>,
    mut event_reader: EventReader<StructureCreated>,
    structure_query: Query<(&Structure, &Transform, &Velocity)>,
    type_query: Query<(Option<&Ship>, Option<&Planet>)>,
) {
    for ev in event_reader.iter() {
        let (structure, transform, velocity) = structure_query.get(ev.entity).unwrap();

        let (ship, planet) = type_query.get(ev.entity).unwrap();

        if planet.is_some() {
            server.broadcast_message(
                NettyChannel::Reliable.id(),
                bincode::serialize(&ServerReliableMessages::PlanetCreate {
                    entity: ev.entity,
                    body: NettyRigidBody::new(velocity, transform),
                    width: structure.chunks_width() as u32,
                    height: structure.chunks_height() as u32,
                    length: structure.chunks_length() as u32,
                })
                .unwrap(),
            );
        } else if ship.is_some() {
            server.broadcast_message(
                NettyChannel::Reliable.id(),
                bincode::serialize(&ServerReliableMessages::ShipCreate {
                    entity: ev.entity,
                    body: NettyRigidBody::new(velocity, transform),
                    width: structure.chunks_width() as u32,
                    height: structure.chunks_height() as u32,
                    length: structure.chunks_length() as u32,
                })
                .unwrap(),
            );
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system_to_stage(CoreStage::PostUpdate, on_structure_created);
}
