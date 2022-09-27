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

struct DelayedStructureCreated {
    pub entity: Entity,
}

fn delayed_add(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut event_reader: EventReader<DelayedStructureCreated>,
    structure_query: Query<(&Structure, &Transform, &Velocity)>,
) {
    for ev in event_reader.iter() {
        let debug_temp = structure_query.get(ev.entity.clone());

        if debug_temp.is_ok() {
            let (structure, transform, velocity) = debug_temp.unwrap();

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
        } else {
            println!("event_listener.rs");
            commands.entity(ev.entity.clone()).log_components();
        }
    }
}

fn on_structure_created(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut event_reader: EventReader<StructureCreated>,
    structure_query: Query<(&Structure, &Transform, &Velocity)>,
    mut evt_writer: EventWriter<DelayedStructureCreated>,
) {
    for ev in event_reader.iter() {
        let debug_temp = structure_query.get(ev.entity.clone());

        if debug_temp.is_ok() {
            let (structure, transform, velocity) = debug_temp.unwrap();

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
        } else {
            println!("event_listener.rs");
            commands.entity(ev.entity.clone()).log_components();

            evt_writer.send(DelayedStructureCreated {
                entity: ev.entity.clone(),
            });
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system(on_structure_created)
        .add_system(delayed_add)
        .add_event::<DelayedStructureCreated>();
}
