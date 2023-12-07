//! Syncs the inventories with the server-provided inventories

use bevy::{
    hierarchy::BuildChildren,
    log::warn,
    prelude::{in_state, App, Commands, Entity, IntoSystemConfigs, Query, Res, ResMut, Update},
};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    block::data::BlockData,
    ecs::NeedsDespawned,
    inventory::netty::ServerInventoryMessages,
    netty::{cosmos_encoder, NettyChannelServer},
    structure::{coordinates::ChunkCoordinate, structure_block::StructureBlock, Structure},
};

use crate::{netty::mapping::NetworkMapping, state::game_state::GameState};

use super::HeldItemStack;

fn sync(
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
    mut commands: Commands,
    mut held_item_query: Query<(Entity, &mut HeldItemStack)>,
    mut structure_query: Query<&mut Structure>,
    mut block_data_query: Query<&mut BlockData>,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::Inventory) {
        let msg: ServerInventoryMessages = cosmos_encoder::deserialize(&message).expect("Failed to deserialize server inventory message!");

        match msg {
            ServerInventoryMessages::EntityInventory { inventory, owner } => {
                if let Some(client_entity) = network_mapping.client_from_server(&owner) {
                    if let Some(mut ecmds) = commands.get_entity(client_entity) {
                        ecmds.insert(inventory);
                    }
                } else {
                    warn!("Error: unrecognized entity {owner:?} received from server when trying to sync up inventories!");
                }
            }
            ServerInventoryMessages::BlockInventory { inventory, block_data } => {
                let Some(client_entity) = network_mapping.client_from_server(&block_data.structure_entity) else {
                    warn!(
                        "Error: unrecognized entity {:?} received from server when trying to sync up inventories!",
                        block_data.structure_entity
                    );
                    continue;
                };

                let Ok(mut structure) = structure_query.get_mut(client_entity) else {
                    continue;
                };

                let coords = block_data.block.coords();

                if let Some(data_entity) = structure.block_data(coords) {
                    if let Ok(mut block_data) = block_data_query.get_mut(data_entity) {
                        if let Some(mut ecmds) = commands.get_entity(data_entity) {
                            block_data.increment();

                            ecmds.insert(inventory);
                        }
                    }
                } else if let Some(chunk_ent) = structure.chunk_entity(ChunkCoordinate::for_block_coordinate(coords)) {
                    if let Some(mut ecmds) = commands.get_entity(chunk_ent) {
                        ecmds.with_children(|p| {
                            let data_entity = p
                                .spawn((
                                    BlockData {
                                        block: StructureBlock::new(coords),
                                        structure_entity: client_entity,
                                        data_count: 1,
                                    },
                                    inventory,
                                ))
                                .id();
                            structure.set_block_data(coords, data_entity);
                        });
                    }
                }
            }
            ServerInventoryMessages::HeldItemstack { itemstack } => {
                if let Ok((entity, mut holding_itemstack)) = held_item_query.get_single_mut() {
                    if let Some(is) = itemstack {
                        // Don't trigger change detection unless it actually changed
                        if is.quantity() != holding_itemstack.quantity() || is.item_id() != holding_itemstack.item_id() {
                            *holding_itemstack = is;
                        }
                    } else {
                        commands.entity(entity).insert(NeedsDespawned);
                    }
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, sync.run_if(in_state(GameState::Playing)));
}
