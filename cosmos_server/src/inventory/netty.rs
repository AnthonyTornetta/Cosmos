//! Syncs player inventories

use bevy::prelude::{in_state, warn, App, Changed, Commands, Entity, IntoSystemConfigs, Query, RemovedComponents, Res, ResMut, Update};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    entities::player::Player,
    inventory::{
        netty::{ClientInventoryMessages, ServerInventoryMessages},
        HeldItemStack, Inventory,
    },
    item::Item,
    netty::{cosmos_encoder, NettyChannelClient, NettyChannelServer},
    registry::Registry,
};

use crate::{netty::network_helpers::ServerLobby, state::GameState};

fn sync_inventories(query: Query<(Entity, &Inventory), Changed<Inventory>>, mut server: ResMut<RenetServer>) {
    for (entity, inventory) in query.iter() {
        server.broadcast_message(
            NettyChannelServer::Inventory,
            cosmos_encoder::serialize(&ServerInventoryMessages::EntityInventory {
                inventory: inventory.clone(),
                owner: entity,
            }),
        );
    }
}

fn sync_held_items(
    query: Query<(&Player, &HeldItemStack), Changed<HeldItemStack>>,
    mut removed_held_itemstacks: RemovedComponents<HeldItemStack>,
    player_query: Query<&Player>,
    mut server: ResMut<RenetServer>,
) {
    for (player, held_itemstack) in query.iter() {
        server.send_message(
            player.id(),
            NettyChannelServer::Inventory,
            cosmos_encoder::serialize(&ServerInventoryMessages::HeldItemstack {
                itemstack: Some(held_itemstack.clone()),
            }),
        );
    }

    for removed_held_item in removed_held_itemstacks.iter() {
        if let Ok(player) = player_query.get(removed_held_item) {
            server.send_message(
                player.id(),
                NettyChannelServer::Inventory,
                cosmos_encoder::serialize(&ServerInventoryMessages::HeldItemstack { itemstack: None }),
            );
        }
    }
}

fn listen(
    mut commands: Commands,
    mut inventory_query: Query<&mut Inventory>,
    mut held_item_query: Query<&mut HeldItemStack>,
    mut server: ResMut<RenetServer>,
    lobby: Res<ServerLobby>,
    items: Res<Registry<Item>>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::Inventory) {
            let Some(client_entity) = lobby.player_from_id(client_id) else {
                continue;
            };

            let msg: ClientInventoryMessages =
                cosmos_encoder::deserialize(&message).expect("Failed to deserialize server inventory message!");

            match msg {
                ClientInventoryMessages::SwapSlots {
                    slot_a,
                    inventory_a,
                    slot_b,
                    inventory_b,
                } => {
                    if inventory_a == inventory_b {
                        if let Ok(mut inventory) = inventory_query.get_mut(inventory_a) {
                            inventory
                                .self_swap_slots(slot_a as usize, slot_b as usize)
                                .unwrap_or_else(|_| panic!("Got bad inventory slots from player! {}, {}", slot_a, slot_b));
                        }
                    } else if let Ok([mut inventory_a, mut inventory_b]) = inventory_query.get_many_mut([inventory_a, inventory_b]) {
                        inventory_a
                            .swap_slots(slot_a as usize, &mut inventory_b, slot_b as usize)
                            .unwrap_or_else(|_| panic!("Got bad inventory slots from player! {}, {}", slot_a, slot_b));
                    }
                }
                ClientInventoryMessages::AutoMove {
                    from_slot,
                    quantity,
                    from_inventory,
                    to_inventory,
                } => {
                    if from_inventory == to_inventory {
                        if let Ok(mut inventory) = inventory_query.get_mut(from_inventory) {
                            inventory
                                .auto_move(from_slot as usize, quantity)
                                .unwrap_or_else(|_| panic!("Got bad inventory slot from player! {}", from_slot));
                        }
                    } else {
                        panic!("Not implemented yet!");
                    }
                }
                ClientInventoryMessages::MoveItemstack {
                    from_slot,
                    quantity,
                    from_inventory,
                    to_inventory,
                    to_slot,
                } => {
                    if from_inventory == to_inventory {
                        if let Ok(mut inventory) = inventory_query.get_mut(from_inventory) {
                            inventory
                                .self_move_itemstack(from_slot as usize, to_slot as usize, quantity)
                                .unwrap_or_else(|_| panic!("Got bad inventory slots from player! {}, {}", from_slot, to_slot));
                        }
                    } else if let Ok([mut inventory_a, mut inventory_b]) = inventory_query.get_many_mut([from_inventory, to_inventory]) {
                        inventory_a
                            .move_itemstack(from_slot as usize, &mut inventory_b, to_slot as usize, quantity)
                            .unwrap_or_else(|_| panic!("Got bad inventory slots from player! {}, {}", from_slot, to_slot));
                    }
                }
                ClientInventoryMessages::PickupItemstack {
                    inventory_holder,
                    slot,
                    quantity,
                } => {
                    let slot = slot as usize;

                    // Check if already holding - if so you can't pick up more stuff
                    if let Ok(is) = held_item_query.get(client_entity) {
                        server.send_message(
                            client_id,
                            NettyChannelServer::Inventory,
                            cosmos_encoder::serialize(&ServerInventoryMessages::HeldItemstack {
                                itemstack: Some(is.clone()),
                            }),
                        );
                        continue;
                    }

                    // TODO: Check if has access to inventory

                    if let Ok(mut inventory) = inventory_query.get_mut(inventory_holder) {
                        if let Some(is) = inventory.mut_itemstack_at(slot) {
                            let quantity = quantity.min(is.quantity());

                            let mut held_itemstack = is.clone();
                            held_itemstack.set_quantity(quantity);
                            // We have confirmed they're not holding anything, so safe to create new entry
                            commands.entity(client_entity).insert(HeldItemStack(held_itemstack));

                            let leftover_quantity = is.quantity() - quantity;
                            is.set_quantity(leftover_quantity);

                            if is.is_empty() {
                                inventory.remove_itemstack_at(slot);
                            }
                        }
                    }
                }
                ClientInventoryMessages::DepositHeldItemstack {
                    inventory_holder,
                    slot,
                    quantity,
                } => {
                    let slot = slot as usize;

                    let Ok(mut held_is) = held_item_query.get_mut(client_entity) else {
                        // Perhaps the client needs updated
                        server.send_message(
                            client_id,
                            NettyChannelServer::Inventory,
                            cosmos_encoder::serialize(&ServerInventoryMessages::HeldItemstack { itemstack: None }),
                        );
                        continue;
                    };

                    // TODO: Check if has access to inventory

                    if let Ok(mut inventory) = inventory_query.get_mut(inventory_holder) {
                        let quantity = quantity.min(held_is.quantity()); // make sure we don't deposit more than we have
                        let unused_quantity = held_is.quantity() - quantity;

                        let leftover = inventory.insert_item_at(slot, items.from_numeric_id(held_is.item_id()), quantity);

                        held_is.set_quantity(unused_quantity + leftover);

                        if held_is.is_empty() {
                            commands.entity(client_entity).remove::<HeldItemStack>();
                        }
                    }
                }
                ClientInventoryMessages::DepositAndSwapHeldItemstack { inventory_holder, slot } => {
                    let slot = slot as usize;

                    let Ok(mut held_item_stack) = held_item_query.get_mut(client_entity) else {
                        // Perhaps the client needs updated
                        server.send_message(
                            client_id,
                            NettyChannelServer::Inventory,
                            cosmos_encoder::serialize(&ServerInventoryMessages::HeldItemstack { itemstack: None }),
                        );
                        continue;
                    };

                    // TODO: Check if has access to inventory

                    if let Ok(mut inventory) = inventory_query.get_mut(inventory_holder) {
                        let itemstack_here = inventory.remove_itemstack_at(slot);

                        let leftover =
                            inventory.insert_item_at(slot, items.from_numeric_id(held_item_stack.item_id()), held_item_stack.quantity());

                        assert_eq!(
                            leftover, 0,
                            "Leftover wasn't 0 somehow? This could only mean something has an invalid stack size"
                        );

                        held_item_stack.set_quantity(0);

                        if let Some(is_here) = itemstack_here {
                            held_item_stack.0 = is_here;
                        } else {
                            commands.entity(client_entity).remove::<HeldItemStack>();
                        }
                    }
                }
                ClientInventoryMessages::ThrowHeldItemstack { quantity } => {
                    let Ok(mut held_item_stack) = held_item_query.get_mut(client_entity) else {
                        // Perhaps the client needs updated
                        server.send_message(
                            client_id,
                            NettyChannelServer::Inventory,
                            cosmos_encoder::serialize(&ServerInventoryMessages::HeldItemstack { itemstack: None }),
                        );
                        continue;
                    };

                    let amount = held_item_stack.quantity().min(quantity);

                    // "Throws" item
                    held_item_stack.decrease_quantity(amount);
                    warn!("Throwing not implemented yet - deleting {amount}.");

                    if held_item_stack.is_empty() {
                        commands.entity(client_entity).remove::<HeldItemStack>();
                    }
                }
                ClientInventoryMessages::InsertHeldItem {
                    quantity,
                    inventory_holder: inventory_entity,
                } => {
                    let Ok(mut held_item_stack) = held_item_query.get_mut(client_entity) else {
                        // Perhaps the client needs updated
                        server.send_message(
                            client_id,
                            NettyChannelServer::Inventory,
                            cosmos_encoder::serialize(&ServerInventoryMessages::HeldItemstack { itemstack: None }),
                        );
                        continue;
                    };

                    let quantity = held_item_stack.quantity().min(quantity);

                    if let Ok(mut inventory) = inventory_query.get_mut(inventory_entity) {
                        let unused_leftover = held_item_stack.quantity() - quantity;
                        let leftover = inventory.insert(items.from_numeric_id(held_item_stack.item_id()), quantity);

                        held_item_stack.set_quantity(leftover + unused_leftover);

                        if held_item_stack.is_empty() {
                            commands.entity(client_entity).remove::<HeldItemStack>();
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (listen, sync_inventories, sync_held_items)
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
