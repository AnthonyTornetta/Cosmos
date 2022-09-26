use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    entities::player::Player,
    netty::{
        client_unreliable_messages::ClientUnreliableMessages, netty::NettyChannel,
        netty_rigidbody::NettyRigidBody,
    },
};

use crate::netty::flags::LocalPlayer;
use crate::state::game_state::GameState;

fn send_position(
    mut client: ResMut<RenetClient>,
    query: Query<(&Velocity, &Transform), (With<Player>, With<LocalPlayer>)>,
) {
    if let Ok((velocity, transform)) = query.get_single() {
        let msg = ClientUnreliableMessages::PlayerBody {
            body: NettyRigidBody::new(&velocity, &transform),
        };

        println!("Sending {}", transform.translation);

        let serialized_message = bincode::serialize(&msg).unwrap();

        client.send_message(NettyChannel::Unreliable.id(), serialized_message);
    }
}

pub fn register(app: &mut App) {
    app.add_system_set(SystemSet::on_update(GameState::Playing).with_system(send_position));
}
