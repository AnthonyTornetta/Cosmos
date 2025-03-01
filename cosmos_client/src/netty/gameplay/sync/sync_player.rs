use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet2::renet2::RenetClient;
use cosmos_core::{
    netty::{
        client::LocalPlayer,
        client_unreliable_messages::ClientUnreliableMessages,
        cosmos_encoder,
        netty_rigidbody::{NettyRigidBody, NettyRigidBodyLocation},
        sync::mapping::NetworkMapping,
        system_sets::NetworkingSystemsSet,
        NettyChannelClient,
    },
    physics::location::{Location, LocationPhysicsSet},
    state::GameState,
};

use crate::rendering::MainCamera;

fn send_position(
    mut client: ResMut<RenetClient>,
    q_player: Query<(&Velocity, &Transform, &Location, Option<&Parent>), With<LocalPlayer>>,
    camera_query: Query<&Transform, With<MainCamera>>,
    netty_mapping: Res<NetworkMapping>,
) {
    if let Ok((velocity, transform, location, parent)) = q_player.get_single() {
        let looking = if let Ok(trans) = camera_query.get_single() {
            Quat::from_affine3(&trans.compute_affine())
        } else {
            Quat::IDENTITY
        };

        let netty_loc = if let Some(parent) = parent.map(|p| p.get()) {
            if let Some(server_ent) = netty_mapping.server_from_client(&parent) {
                NettyRigidBodyLocation::Relative(transform.translation, server_ent)
            } else {
                NettyRigidBodyLocation::Absolute(*location)
            }
        } else {
            NettyRigidBodyLocation::Absolute(*location)
        };

        let msg = ClientUnreliableMessages::PlayerBody {
            body: NettyRigidBody::new(Some(*velocity), transform.rotation, netty_loc),
            looking,
        };

        let serialized_message = cosmos_encoder::serialize(&msg);

        client.send_message(NettyChannelClient::Unreliable, serialized_message);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        send_position
            .in_set(NetworkingSystemsSet::SyncComponents)
            .after(LocationPhysicsSet::DoPhysics)
            .run_if(in_state(GameState::Playing)),
    );
}
