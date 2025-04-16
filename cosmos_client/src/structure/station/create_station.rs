//! Event & its processing for when a player wants to create a station

use bevy::prelude::{App, Event, EventReader, EventWriter, IntoSystemConfigs, Or, Query, ResMut, Update, With, in_state};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{NettyChannelClient, client::LocalPlayer, client_reliable_messages::ClientReliableMessages, cosmos_encoder},
    state::GameState,
    structure::{shared::build_mode::BuildMode, ship::pilot::Pilot},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::components::show_cursor::no_open_menus,
};

#[derive(Debug, Event)]
/// Sent when the client wants the server to create a station
pub struct CreateStationEvent {
    name: String,
}

fn listener(
    q_invalid_player: Query<(), (With<LocalPlayer>, Or<(With<BuildMode>, With<Pilot>)>)>,
    input_handler: InputChecker,
    mut event_writer: EventWriter<CreateStationEvent>,
) {
    if q_invalid_player.get_single().is_ok() {
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
