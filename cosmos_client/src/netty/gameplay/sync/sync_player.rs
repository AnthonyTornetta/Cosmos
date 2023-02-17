use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{
        client_unreliable_messages::ClientUnreliableMessages, netty_rigidbody::NettyRigidBody,
        NettyChannel,
    },
    physics::location::Location,
};

use crate::netty::flags::LocalPlayer;
use crate::state::game_state::GameState;

fn send_position(
    mut client: ResMut<RenetClient>,
    query: Query<(&Velocity, &Transform, &Location), With<LocalPlayer>>,
    camera_query: Query<&Transform, With<Camera3d>>,
) {
    if let Ok((velocity, transform, location)) = query.get_single() {
        let looking = if let Ok(trans) = camera_query.get_single() {
            Quat::from_affine3(&trans.compute_affine())
        } else {
            Quat::IDENTITY
        };

        let msg = ClientUnreliableMessages::PlayerBody {
            body: NettyRigidBody::new(velocity, transform.rotation, *location),
            looking,
        };

        let serialized_message = bincode::serialize(&msg).unwrap();

        client.send_message(NettyChannel::Unreliable.id(), serialized_message);
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system_set(SystemSet::on_update(GameState::Playing).with_system(send_position));
}
