use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::{transport::NetcodeClientTransport, RenetClient};
use cosmos_core::{
    netty::{
        client::LocalPlayer,
        client_unreliable_messages::ClientUnreliableMessages,
        cosmos_encoder,
        netty_rigidbody::{NettyRigidBody, NettyRigidBodyLocation},
        sync::mapping::NetworkMapping,
        NettyChannelClient,
    },
    physics::location::Location,
};

use crate::input::inputs::{CosmosInputs, InputHandler};
use crate::{input::inputs::InputChecker, rendering::MainCamera, state::game_state::GameState};

fn send_position(
    mut client: ResMut<RenetClient>,
    loc_query: Query<&Location>,
    query: Query<(&Velocity, &Transform, &Location, Option<&Parent>), With<LocalPlayer>>,
    camera_query: Query<&Transform, With<MainCamera>>,
    netty_mapping: Res<NetworkMapping>,
) {
    if let Ok((velocity, transform, location, parent)) = query.get_single() {
        let looking = if let Ok(trans) = camera_query.get_single() {
            Quat::from_affine3(&trans.compute_affine())
        } else {
            Quat::IDENTITY
        };

        let netty_loc = if let Some(parent) = parent.map(|p| p.get()) {
            if let Some(server_ent) = netty_mapping.server_from_client(&parent) {
                let parent_loc = loc_query.get(parent).copied().unwrap_or(Location::default());

                NettyRigidBodyLocation::Relative((*location - parent_loc).absolute_coords_f32(), server_ent)
            } else {
                NettyRigidBodyLocation::Absolute(*location)
            }
        } else {
            NettyRigidBodyLocation::Absolute(*location)
        };

        let msg = ClientUnreliableMessages::PlayerBody {
            body: NettyRigidBody::new(velocity, transform.rotation, netty_loc),
            looking,
        };

        let serialized_message = cosmos_encoder::serialize(&msg);

        client.send_message(NettyChannelClient::Unreliable, serialized_message);
    }
}

// Just for testing
fn send_disconnect(input_handler: InputChecker, transport: Option<ResMut<NetcodeClientTransport>>, client: Res<RenetClient>) {
    if input_handler.check_just_pressed(CosmosInputs::Disconnect) {
        if let Some(mut transport) = transport {
            if client.is_connected() {
                info!("SENDING DC MESSAGE!");
                transport.disconnect();
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (send_position, send_disconnect).run_if(in_state(GameState::Playing)));
}
