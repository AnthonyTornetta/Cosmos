use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{
        netty_rigidbody::NettyRigidBody, server_reliable_messages::ServerReliableMessages,
        NettyChannel,
    },
    physics::location::Location,
    structure::{planet::Planet, ship::Ship, Structure},
};

fn on_structure_created(
    mut server: ResMut<RenetServer>,
    structure_query: Query<
        (
            Entity,
            &Structure,
            &Transform,
            &Velocity,
            &Location,
            Option<&Planet>,
            Option<&Ship>,
        ),
        Added<Structure>,
    >,
) {
    for (entity, structure, transform, velocity, location, is_planet, is_ship) in
        structure_query.iter()
    {
        if is_planet.is_some() {
            server.broadcast_message(
                NettyChannel::Reliable.id(),
                bincode::serialize(&ServerReliableMessages::PlanetCreate {
                    entity,
                    body: NettyRigidBody::new(velocity, transform.rotation, *location),
                    width: structure.chunks_width() as u32,
                    height: structure.chunks_height() as u32,
                    length: structure.chunks_length() as u32,
                })
                .unwrap(),
            );
        } else if is_ship.is_some() {
            server.broadcast_message(
                NettyChannel::Reliable.id(),
                bincode::serialize(&ServerReliableMessages::ShipCreate {
                    entity,
                    body: NettyRigidBody::new(velocity, transform.rotation, *location),
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
    app.add_system(on_structure_created);
}
