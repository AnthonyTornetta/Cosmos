use bevy::prelude::{
    App, EventReader, EventWriter, Input, KeyCode, MouseButton, Res, ResMut, SystemSet,
};
use bevy_renet::renet::RenetClient;
use cosmos_core::netty::{client_reliable_messages::ClientReliableMessages, netty::NettyChannel};

use crate::{
    input::inputs::{CosmosInputHandler, CosmosInputs},
    state::game_state::GameState,
};

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
        event_writer.send(CreateShipEvent {
            name: "Cool name".into(),
        });
    }
}

fn event_handler(mut event_reader: EventReader<CreateShipEvent>, mut client: ResMut<RenetClient>) {
    for ev in event_reader.iter() {
        client.send_message(
            NettyChannel::Reliable.id(),
            bincode::serialize(&ClientReliableMessages::CreateShip {
                name: ev.name.clone(),
            })
            .unwrap(),
        );
    }
}

pub fn register(app: &mut App) {
    app.add_event::<CreateShipEvent>();
    app.add_system_set(
        SystemSet::on_update(GameState::Playing)
            .with_system(event_handler)
            .with_system(listener),
    );
}
