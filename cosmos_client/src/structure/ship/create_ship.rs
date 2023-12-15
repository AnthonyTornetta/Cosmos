//! Event & its processing for when a player wants to create a ship

use bevy::prelude::{in_state, App, Event, EventReader, EventWriter, IntoSystemConfigs, Query, ResMut, Update, With};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient},
    structure::shared::build_mode::BuildMode,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    netty::flags::LocalPlayer,
    state::game_state::GameState,
};

#[derive(Debug, Event)]
/// Sent when the client wants the server to create a ship
pub struct CreateShipEvent {
    name: String,
}

fn listener(
    in_build_mode: Query<(), (With<LocalPlayer>, With<BuildMode>)>,
    input_handler: InputChecker,
    mut event_writer: EventWriter<CreateShipEvent>,
) {
    if in_build_mode.get_single().is_ok() {
        // Don't create ships while in build mode
        return;
    }

    if input_handler.check_just_pressed(CosmosInputs::CreateShip) {
        event_writer.send(CreateShipEvent { name: "Cool name".into() });
    }
}

fn event_handler(mut event_reader: EventReader<CreateShipEvent>, mut client: ResMut<RenetClient>) {
    for ev in event_reader.read() {
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::CreateShip { name: ev.name.clone() }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<CreateShipEvent>()
        .add_systems(Update, (listener, event_handler).chain().run_if(in_state(GameState::Playing)));
}
