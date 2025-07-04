//! Syncs the inventories with the server-provided inventories

use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    inventory::{
        Inventory,
        netty::{InventoryIdentifier, ServerInventoryMessages},
    },
    netty::{NettyChannelServer, client::LocalPlayer, cosmos_encoder, sync::mapping::NetworkMapping, system_sets::NetworkingSystemsSet},
    state::GameState,
    structure::Structure,
};

use super::{InventoryNeedsDisplayed, InventorySide};

fn sync(
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
    mut commands: Commands,
    structure_query: Query<&Structure>,
    local_player: Query<Entity, With<LocalPlayer>>,
    q_check_inventory: Query<(), With<Inventory>>,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::Inventory) {
        let msg: ServerInventoryMessages = cosmos_encoder::deserialize(&message).expect("Failed to deserialize server inventory message!");

        match msg {
            ServerInventoryMessages::OpenInventory { owner } => {
                match owner {
                    InventoryIdentifier::Entity(owner) => {
                        if let Some(client_entity) = network_mapping.client_from_server(&owner) {
                            if let Ok(mut ecmds) = commands.get_entity(client_entity) {
                                ecmds.insert(InventoryNeedsDisplayed::default());
                            }
                        } else {
                            warn!("Error: unrecognized entity {owner:?} received from server when trying to sync up inventories!");
                        }
                    }
                    InventoryIdentifier::BlockData(block_data) => {
                        let Some(client_entity) = network_mapping.client_from_server(&block_data.block.structure()) else {
                            warn!(
                                "Error: unrecognized entity {:?} received from server when trying to sync up inventories!",
                                block_data.block.structure()
                            );
                            continue;
                        };

                        let Ok(structure) = structure_query.get(client_entity) else {
                            warn!("Tried to open inventory of unknown structure");
                            continue;
                        };

                        let coords = block_data.block.coords();

                        let Some(data_entity) = structure.block_data(coords) else {
                            warn!("Tried to open inventory of block without any client-side block data.");
                            continue;
                        };

                        if !q_check_inventory.contains(data_entity) {
                            warn!("Tried to open inventory of block with block data but without an inventory component!");
                            continue;
                        }

                        commands.entity(data_entity).insert(InventoryNeedsDisplayed::default());
                    }
                }

                commands
                    .entity(local_player.single().expect("Missing local player"))
                    .insert(InventoryNeedsDisplayed::Normal(InventorySide::Left));
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        // TODO: This really shouldn't be done manually here, in the future this should be automatically
        // synced using some sort of easy server events framework
        sync.in_set(NetworkingSystemsSet::SyncComponents)
            .ambiguous_with(NetworkingSystemsSet::SyncComponents)
            .run_if(in_state(GameState::Playing)),
    );
}
