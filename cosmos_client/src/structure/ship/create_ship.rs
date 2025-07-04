//! Event & its processing for when a player wants to create a ship

use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    block::block_events::BlockEventsSet,
    inventory::Inventory,
    item::Item,
    netty::{NettyChannelClient, client::LocalPlayer, client_reliable_messages::ClientReliableMessages, cosmos_encoder},
    registry::Registry,
    state::GameState,
    structure::{shared::build_mode::BuildMode, ship::pilot::Pilot},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::components::show_cursor::no_open_menus,
};

#[derive(Debug, Event)]
/// Sent when the client wants the server to create a ship
pub struct CreateShipEvent {
    name: String,
}

fn listener(
    q_inventory: Query<&Inventory, (With<LocalPlayer>, Without<BuildMode>, Without<Pilot>)>,
    items: Res<Registry<Item>>,
    input_handler: InputChecker,
    mut event_writer: EventWriter<CreateShipEvent>,
) {
    // Don't create ships while in build mode
    let Ok(inventory) = q_inventory.single() else {
        return;
    };

    if input_handler.check_just_pressed(CosmosInputs::CreateShip) {
        let Some(ship_core) = items.from_id("cosmos:ship_core") else {
            info!("Ship core not registered");
            return;
        };

        if inventory.can_take_item(ship_core, 1) {
            info!("Sending create ship event!");
            event_writer.write(CreateShipEvent { name: "Cool name".into() });
        } else {
            info!("Does not have ship core");
        }
    }
}

fn event_handler(mut event_reader: EventReader<CreateShipEvent>, mut client: ResMut<RenetClient>) {
    for ev in event_reader.read() {
        info!("Got create ship event!");
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::CreateShip { name: ev.name.clone() }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<CreateShipEvent>().add_systems(
        Update,
        (listener.run_if(no_open_menus), event_handler)
            .in_set(BlockEventsSet::SendEventsForNextFrame)
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
