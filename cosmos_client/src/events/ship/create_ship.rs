//! Event & its processing for when a player wants to create a ship

use bevy::prelude::{App, EventReader, EventWriter, Input, IntoSystemConfigs, KeyCode, MouseButton, OnUpdate, Res, ResMut};
use bevy_renet::renet::RenetClient;
use cosmos_core::netty::{client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient};

use crate::{
    input::inputs::{CosmosInputHandler, CosmosInputs},
    state::game_state::GameState,
};

#[derive(Debug)]
/// Sent when the client wants the server to create a ship
pub struct CreateShipEvent {
    name: String,
}

fn listener(
    inputs: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    cosmos_inputs: Res<CosmosInputHandler>,
    mut event_writer: EventWriter<CreateShipEvent>,
) {
    if cosmos_inputs.check_just_pressed(CosmosInputs::CreateShip, &inputs, &mouse) {
        event_writer.send(CreateShipEvent { name: "Cool name".into() });
    }
}

fn event_handler(mut event_reader: EventReader<CreateShipEvent>, mut client: ResMut<RenetClient>) {
    for ev in event_reader.iter() {
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::CreateShip { name: ev.name.clone() }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<CreateShipEvent>()
        .add_systems((event_handler, listener).in_set(OnUpdate(GameState::Playing)));
}
