use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{
        netty::NettyChannel, netty_rigidbody::NettyRigidBody,
        server_reliable_messages::ServerReliableMessages,
    },
    structure::{events::StructureCreated, structure::Structure},
};

fn on_structure_created(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut event_reader: EventReader<StructureCreated>,
    structure_query: Query<(&Structure, &Transform, &Velocity)>,
) {
    for ev in event_reader.iter() {
        let debug_temp = structure_query.get(ev.entity.clone());

        if debug_temp.is_ok() {
            let (structure, transform, velocity) = debug_temp.unwrap();

            server.broadcast_message(
                NettyChannel::Reliable.id(),
                bincode::serialize(&ServerReliableMessages::StructureCreate {
                    entity: ev.entity.clone(),
                    body: NettyRigidBody::new(velocity, transform),
                    width: structure.chunks_width(),
                    height: structure.chunks_height(),
                    length: structure.chunks_length(),
                })
                .unwrap(),
            );
        } else {
            println!("event_listener.rs");
            commands.entity(ev.entity.clone()).log_components();
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system(on_structure_created);
}
