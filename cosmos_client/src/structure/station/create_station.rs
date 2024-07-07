//! Event & its processing for when a player wants to create a station

use bevy::prelude::{in_state, App, Event, EventReader, EventWriter, IntoSystemConfigs, Query, ResMut, Update, With};
use bevy_renet2::renet2::RenetClient;
use cosmos_core::{
    netty::{client::LocalPlayer, client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient},
    structure::shared::build_mode::BuildMode,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    state::game_state::GameState,
    ui::components::show_cursor::no_open_menus,
};

#[derive(Debug, Event)]
/// Sent when the client wants the server to create a station
pub struct CreateStationEvent {
    name: String,
}

fn listener(
    in_build_mode: Query<(), (With<LocalPlayer>, With<BuildMode>)>,
    input_handler: InputChecker,
    mut event_writer: EventWriter<CreateStationEvent>,
) {
    if in_build_mode.get_single().is_ok() {
        // Don't create stations while in build mode
        return;
    }

    if input_handler.check_just_pressed(CosmosInputs::CreateStation) {
        event_writer.send(CreateStationEvent { name: "Cool name".into() });
    }
}

fn event_handler(mut event_reader: EventReader<CreateStationEvent>, mut client: ResMut<RenetClient>) {
    for ev in event_reader.read() {
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::CreateStation { name: ev.name.clone() }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<CreateStationEvent>().add_systems(
        Update,
        (listener.run_if(no_open_menus), event_handler)
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
