use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{
        netty::NettyChannel, netty_rigidbody::NettyRigidBody,
        server_reliable_messages::ServerReliableMessages,
    },
    structure::{
        events::StructureCreated, planet::planet::Planet, ship::ship::Ship, structure::Structure,
    },
};

fn on_structure_created(
    mut server: ResMut<RenetServer>,
    mut event_reader: EventReader<StructureCreated>,
    structure_query: Query<(&Structure, &Transform, &Velocity)>,
    type_query: Query<(Option<&Ship>, Option<&Planet>)>,
) {
    for ev in event_reader.iter() {
        let (structure, transform, velocity) = structure_query.get(ev.entity.clone()).unwrap();

        let (ship, planet) = type_query.get(ev.entity.clone()).unwrap();

        if planet.is_some() {
            server.broadcast_message(
                NettyChannel::Reliable.id(),
                bincode::serialize(&ServerReliableMessages::PlanetCreate {
                    entity: ev.entity.clone(),
                    body: NettyRigidBody::new(velocity, transform),
                    width: structure.chunks_width(),
                    height: structure.chunks_height(),
                    length: structure.chunks_length(),
                })
                .unwrap(),
            );
        } else if ship.is_some() {
            server.broadcast_message(
                NettyChannel::Reliable.id(),
                bincode::serialize(&ServerReliableMessages::ShipCreate {
                    entity: ev.entity.clone(),
                    body: NettyRigidBody::new(velocity, transform),
                    width: structure.chunks_width(),
                    height: structure.chunks_height(),
                    length: structure.chunks_length(),
                })
                .unwrap(),
            );
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system_to_stage(CoreStage::PostUpdate, on_structure_created);
}
