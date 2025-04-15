//! Syncs the inventories with the server-provided inventories

use bevy::{
    ecs::query::With,
    log::warn,
    prelude::{in_state, App, Commands, Entity, IntoSystemConfigs, Query, Res, ResMut, Update},
};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    ecs::NeedsDespawned,
    inventory::{
        netty::{InventoryIdentifier, ServerInventoryMessages},
        Inventory,
    },
    netty::{client::LocalPlayer, cosmos_encoder, sync::mapping::NetworkMapping, system_sets::NetworkingSystemsSet, NettyChannelServer},
    state::GameState,
    structure::Structure,
};

use super::{HeldItemStack, InventoryNeedsDisplayed, InventorySide};

fn sync(
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
    mut commands: Commands,
    mut held_item_query: Query<(Entity, &mut HeldItemStack)>,
    structure_query: Query<&Structure>,
    local_player: Query<Entity, With<LocalPlayer>>,
    q_check_inventory: Query<(), With<Inventory>>,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::Inventory) {
        let msg: ServerInventoryMessages = cosmos_encoder::deserialize(&message).expect("Failed to deserialize server inventory message!");

        match msg {
            ServerInventoryMessages::HeldItemstack { itemstack } => {
                if let Ok((entity, mut holding_itemstack)) = held_item_query.get_single_mut() {
                    if let Some(mut is) = itemstack {
                        // Don't trigger change detection unless it actually changed
                        if is.quantity() != holding_itemstack.quantity() || is.item_id() != holding_itemstack.item_id() {
                            if let Some(de) = is.data_entity() {
                                if let Some(de) = network_mapping.client_from_server(&de) {
                                    is.set_data_entity(Some(de));
                                } else {
                                    warn!("Missing data entity for is!");
                                    is.set_data_entity(None);
                                }
                            }

                            *holding_itemstack = is;
                        }
                    } else {
                        commands.entity(entity).insert(NeedsDespawned);
                    }
                }
            }
            ServerInventoryMessages::OpenInventory { owner } => {
                match owner {
                    InventoryIdentifier::Entity(owner) => {
                        if let Some(client_entity) = network_mapping.client_from_server(&owner) {
                            if let Some(mut ecmds) = commands.get_entity(client_entity) {
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
                    .entity(local_player.single())
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
