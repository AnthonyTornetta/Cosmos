//! Syncs the inventories with the server-provided inventories

use bevy::prelude::{in_state, App, Commands, IntoSystemConfigs, Res, ResMut, Update};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    inventory::netty::ServerInventoryMessages,
    netty::{cosmos_encoder, NettyChannelServer},
};

use crate::{netty::mapping::NetworkMapping, state::game_state::GameState};

fn sync(mut client: ResMut<RenetClient>, network_mapping: Res<NetworkMapping>, mut commands: Commands) {
    while let Some(message) = client.receive_message(NettyChannelServer::Inventory) {
        let msg: ServerInventoryMessages = cosmos_encoder::deserialize(&message).expect("Failed to deserialize server inventory message!");

        match msg {
            ServerInventoryMessages::EntityInventory { inventory, owner } => {
                if let Some(client_entity) = network_mapping.client_from_server(&owner) {
                    if let Some(mut ecmds) = commands.get_entity(client_entity) {
                        ecmds.insert(inventory);
                    }
                } else {
                    eprintln!("Error: unrecognized entity {} received from server!", owner.index());
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, sync.run_if(in_state(GameState::Playing)));
}
