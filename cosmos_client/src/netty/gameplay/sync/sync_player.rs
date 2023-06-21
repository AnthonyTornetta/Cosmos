use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::{transport::NetcodeClientTransport, RenetClient};
use cosmos_core::{
    netty::{
        client_unreliable_messages::ClientUnreliableMessages, cosmos_encoder,
        netty_rigidbody::NettyRigidBody, NettyChannelClient,
    },
    physics::location::Location,
};

use crate::{
    input::inputs::CosmosInputHandler, rendering::MainCamera, state::game_state::GameState,
};
use crate::{input::inputs::CosmosInputs, netty::flags::LocalPlayer};

fn send_position(
    mut client: ResMut<RenetClient>,
    query: Query<(&Velocity, &Transform, &Location), With<LocalPlayer>>,
    camera_query: Query<&Transform, With<MainCamera>>,
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

        let serialized_message = cosmos_encoder::serialize(&msg);

        client.send_message(NettyChannelClient::Unreliable, serialized_message);
    }
}

// Just for testing
fn send_disconnect(
    inputs: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    input_handler: ResMut<CosmosInputHandler>,
    transport: Option<ResMut<NetcodeClientTransport>>,
) {
    if input_handler.check_just_pressed(CosmosInputs::Disconnect, &inputs, &mouse) {
        if let Some(mut transport) = transport {
            if transport.is_connected() {
                println!("SENDING DC MESSAGE!");
                transport.disconnect();
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems((send_position, send_disconnect).in_set(OnUpdate(GameState::Playing)));
}
