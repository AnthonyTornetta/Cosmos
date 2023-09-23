//! Event & its processing for when a player wants to create a ship

use bevy::prelude::{in_state, App, Event, EventReader, EventWriter, IntoSystemConfigs, ResMut, Update};
use bevy_renet::renet::RenetClient;
use cosmos_core::netty::{client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    state::game_state::GameState,
};

#[derive(Debug, Event)]
/// Sent when the client wants the server to create a ship
pub struct CreateShipEvent {
    name: String,
}

fn listener(input_handler: InputChecker, mut event_writer: EventWriter<CreateShipEvent>) {
    if input_handler.check_just_pressed(CosmosInputs::CreateShip) {
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
        .add_systems(Update, (event_handler, listener).run_if(in_state(GameState::Playing)));
}
