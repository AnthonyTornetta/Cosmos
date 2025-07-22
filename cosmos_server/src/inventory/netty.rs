//! Syncs player inventories

use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    inventory::{
        HeldItemStack, Inventory,
        netty::{ClientInventoryMessages, InventoryIdentifier},
    },
    item::physical_item::PhysicalItem,
    netty::{NettyChannelClient, cosmos_encoder, server::ServerLobby, system_sets::NetworkingSystemsSet},
    physics::location::Location,
    state::GameState,
    structure::{Structure, ship::pilot::Pilot},
};

use crate::{
    entities::player::PlayerLooking,
    inventory::{InventoryAddItemEvent, InventoryRemoveItemEvent, MovedItem},
};

fn get_inventory_mut<'a>(
    identifier: InventoryIdentifier,
    q_inventory: &'a mut Query<&mut Inventory, (Without<Pilot>, Without<HeldItemStack>)>,
    q_structure: &'a Query<&Structure>,
) -> Option<(Entity, Mut<'a, Inventory>)> {
    match identifier {
        InventoryIdentifier::Entity(entity) => q_inventory.get_mut(entity).ok().map(|i| (entity, i)),
        InventoryIdentifier::BlockData(block_data) => {
            let Ok(structure) = q_structure.get(block_data.block.structure()) else {
                warn!("Missing structure entity for {:?}", block_data.block.structure());
                return None;
            };

            let Some(block_data_ent) = structure.block_data(block_data.block.coords()) else {
                warn!(
                    "Missing block data for {} in entity {:?}",
                    block_data.block.coords(),
                    block_data.block.structure()
                );
                return None;
            };

            q_inventory.get_mut(block_data_ent).ok().map(|bd| (block_data_ent, bd))
        }
    }
}

fn get_many_inventories_mut<'a, const N: usize>(
    identifiers: [InventoryIdentifier; N],
    q_inventory: &'a mut Query<&mut Inventory, (Without<Pilot>, Without<HeldItemStack>)>,
    q_structure: &'a Query<&Structure>,
) -> Option<([Mut<'a, Inventory>; N], [Entity; N])> {
    let ents = identifiers
        .into_iter()
        .map(|x| match x {
            InventoryIdentifier::Entity(entity) => Some(entity),
            InventoryIdentifier::BlockData(block_data) => {
                let structure = q_structure.get(block_data.block.structure()).ok()?;

                structure.block_data(block_data.block.coords())
            }
        })
        .take_while(|x| x.is_some())
        .map(|x| x.unwrap())
        .collect::<Vec<Entity>>();

    // If this try_into fails, one of the entities in the above iter was None.
    let ents = ents.try_into().ok()?;
    let inventories = q_inventory.get_many_mut(ents).ok()?;

    Some((inventories, ents))
}

fn listen_for_inventory_messages(
    mut commands: Commands,
    mut q_inventory: Query<&mut Inventory, (Without<Pilot>, Without<HeldItemStack>)>,
    q_structure: Query<&Structure>,
    mut q_held_item: Query<&mut Inventory, With<HeldItemStack>>,
    mut server: ResMut<RenetServer>,
    q_player: Query<(&Location, &GlobalTransform, &PlayerLooking, &Velocity)>,
    q_children: Query<&Children>,
    lobby: Res<ServerLobby>,
    mut evw_add_item: EventWriter<InventoryAddItemEvent>,
    mut evw_remove_item: EventWriter<InventoryRemoveItemEvent>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::Inventory) {
            let Some(client_entity) = lobby.player_from_id(client_id) else {
                continue;
            };

            let Ok(msg) = cosmos_encoder::deserialize::<ClientInventoryMessages>(&message).map_err(|e| {
                error!("{e:?}");
                e
            }) else {
                error!("Failed to deserialize server inventory message!");
                continue;
            };

            match msg {
                ClientInventoryMessages::SwapSlots {
                    slot_a,
                    inventory_a,
                    slot_b,
                    inventory_b,
                } => {
                    if inventory_a == inventory_b {
                        if let Some((_, mut inventory)) = get_inventory_mut(inventory_a, &mut q_inventory, &q_structure) {
                            inventory
                                .self_swap_slots(slot_a as usize, slot_b as usize, &mut commands)
                                .unwrap_or_else(|_| panic!("Got bad inventory slots from player! {slot_a}, {slot_b}"));
                        }
                    } else if let Some(([mut inventory_a, mut inventory_b], [a, b])) =
                        get_many_inventories_mut([inventory_a, inventory_b], &mut q_inventory, &q_structure)
                    {
                        let b_item = inventory_b.itemstack_at(slot_b as usize);
                        if let Some(is) = b_item {
                            evw_add_item.write(InventoryAddItemEvent {
                                inventory_entity: a,
                                item: MovedItem {
                                    amount: is.quantity(),
                                    slot: slot_a,
                                    item_id: is.item_id(),
                                },
                                adder: Some(client_entity),
                            });
                            evw_remove_item.write(InventoryRemoveItemEvent {
                                inventory_entity: b,
                                item: MovedItem {
                                    amount: is.quantity(),
                                    slot: slot_b,
                                    item_id: is.item_id(),
                                },
                                remover: Some(client_entity),
                            });
                        }
                        let a_item = inventory_a.itemstack_at(slot_a as usize);
                        if let Some(is) = a_item {
                            evw_add_item.write(InventoryAddItemEvent {
                                inventory_entity: b,
                                item: MovedItem {
                                    amount: is.quantity(),
                                    slot: slot_b,
                                    item_id: is.item_id(),
                                },
                                adder: Some(client_entity),
                            });
                            evw_remove_item.write(InventoryRemoveItemEvent {
                                inventory_entity: a,
                                item: MovedItem {
                                    amount: is.quantity(),
                                    slot: slot_a,
                                    item_id: is.item_id(),
                                },
                                remover: Some(client_entity),
                            });
                        }

                        inventory_a
                            .swap_slots(slot_a as usize, &mut inventory_b, slot_b as usize, &mut commands)
                            .unwrap_or_else(|_| panic!("Got bad inventory slots from player! {slot_a}, {slot_b}"));
                    }
                }
                ClientInventoryMessages::AutoMove {
                    from_slot,
                    quantity,
                    from_inventory,
                    to_inventory,
                } => {
                    if from_inventory == to_inventory {
                        if let Some((_, mut inventory)) = get_inventory_mut(from_inventory, &mut q_inventory, &q_structure) {
                            inventory
                                .auto_move(from_slot as usize, quantity, &mut commands)
                                .unwrap_or_else(|_| panic!("Got bad inventory slot from player! {from_slot}"));
                        }
                    } else if let Some(([mut from_inventory, mut to_inventory], [from_ent, to_ent])) =
                        get_many_inventories_mut([from_inventory, to_inventory], &mut q_inventory, &q_structure)
                    {
                        let from_slot = from_slot as usize;
                        if let Some(mut is) = from_inventory.remove_itemstack_at(from_slot) {
                            let (leftover, moved_to) = to_inventory.insert_itemstack(&is, &mut commands);
                            if let Some(moved_to) = moved_to {
                                evw_add_item.write(InventoryAddItemEvent {
                                    inventory_entity: to_ent,
                                    item: MovedItem {
                                        amount: is.quantity() - leftover,
                                        slot: moved_to as u32,
                                        item_id: is.item_id(),
                                    },
                                    adder: Some(client_entity),
                                });
                                evw_remove_item.write(InventoryRemoveItemEvent {
                                    inventory_entity: from_ent,
                                    item: MovedItem {
                                        amount: is.quantity() - leftover,
                                        slot: from_slot as u32,
                                        item_id: is.item_id(),
                                    },
                                    remover: Some(client_entity),
                                });
                            }

                            if leftover == 0 {
                                from_inventory.remove_itemstack_at(from_slot);
                            } else if leftover == is.quantity() {
                                from_inventory.set_itemstack_at(from_slot, Some(is), &mut commands);

                                from_inventory
                                    .auto_move(from_slot, quantity, &mut commands)
                                    .unwrap_or_else(|_| warn!("Got bad inventory slot from player! {from_slot}"));
                            } else {
                                is.set_quantity(leftover);
                                from_inventory.set_itemstack_at(from_slot, Some(is), &mut commands);
                            }
                        }
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
                        if let Some((_, mut inventory)) = get_inventory_mut(from_inventory, &mut q_inventory, &q_structure) {
                            inventory
                                .self_move_itemstack(from_slot as usize, to_slot as usize, quantity, &mut commands)
                                .unwrap_or_else(|_| panic!("Got bad inventory slots from player! {from_slot}, {to_slot}"));
                        }
                    } else if let Some(([mut inventory_a, mut inventory_b], [from, to])) =
                        get_many_inventories_mut([from_inventory, to_inventory], &mut q_inventory, &q_structure)
                        && let Some(is) = inventory_a.itemstack_at(from_slot as usize) {
                            let qty = is.quantity();
                            let item_id = is.item_id();
                            if let Ok(leftover) =
                                inventory_a.move_itemstack(from_slot as usize, &mut inventory_b, to_slot as usize, quantity, &mut commands)
                            {
                                let amount = qty - leftover;
                                evw_add_item.write(InventoryAddItemEvent {
                                    inventory_entity: to,
                                    item: MovedItem {
                                        amount,
                                        slot: to_slot,
                                        item_id,
                                    },
                                    adder: Some(client_entity),
                                });
                                evw_remove_item.write(InventoryRemoveItemEvent {
                                    inventory_entity: from,
                                    item: MovedItem {
                                        amount,
                                        slot: from_slot,
                                        item_id,
                                    },
                                    remover: Some(client_entity),
                                });
                            }
                        }
                }
                ClientInventoryMessages::PickupItemstack {
                    inventory_holder,
                    slot,
                    quantity,
                } => {
                    let slot = slot as usize;

                    let Some(mut held_item_inv) = HeldItemStack::get_held_is_inventory_mut(client_entity, &q_children, &mut q_held_item)
                    else {
                        continue;
                    };

                    // Check if already holding - if so you can't pick up more stuff
                    if held_item_inv.itemstack_at(0).is_some() {
                        continue;
                    }

                    // TODO: Check if has access to inventory

                    if let Some((inv_ent, mut inventory)) = get_inventory_mut(inventory_holder, &mut q_inventory, &q_structure)
                        && let Some(is) = inventory.mut_itemstack_at(slot)
                    {
                        let quantity = quantity.min(is.quantity());

                        evw_remove_item.write(InventoryRemoveItemEvent {
                            inventory_entity: inv_ent,
                            item: MovedItem {
                                amount: quantity,
                                slot: slot as u32,
                                item_id: is.item_id(),
                            },
                            remover: Some(client_entity),
                        });

                        let mut held_itemstack = is.clone();
                        held_itemstack.set_quantity(quantity);

                        held_item_inv.set_itemstack_at(0, Some(held_itemstack), &mut commands);
                        // We have confirmed they're not holding anything, so safe to create new entry
                        // commands.entity(client_entity).insert(HeldItemStack(held_itemstack));

                        let leftover_quantity = is.quantity() - quantity;
                        is.set_quantity(leftover_quantity);

                        if is.is_empty() {
                            inventory.remove_itemstack_at(slot);
                        }
                    }
                }
                ClientInventoryMessages::DepositHeldItemstack {
                    inventory_holder,
                    slot,
                    quantity,
                } => {
                    let slot = slot as usize;

                    let Some(mut held_item_inv) = HeldItemStack::get_held_is_inventory_mut(client_entity, &q_children, &mut q_held_item)
                    else {
                        continue;
                    };
                    let Some(mut held_is) = held_item_inv.remove_itemstack_at(0) else {
                        continue;
                    };

                    // TODO: Check if has access to inventory

                    if let Some((inv_ent, mut inventory)) = get_inventory_mut(inventory_holder, &mut q_inventory, &q_structure) {
                        let quantity = quantity.min(held_is.quantity()); // make sure we don't deposit more than we have
                        let mut moving_is = held_is.clone();
                        moving_is.set_quantity(quantity);

                        let unused_quantity = held_is.quantity() - quantity;

                        let leftover = inventory.insert_itemstack_at(slot, &moving_is, &mut commands);

                        evw_add_item.write(InventoryAddItemEvent {
                            inventory_entity: inv_ent,
                            item: MovedItem {
                                amount: quantity - leftover,
                                slot: slot as u32,
                                item_id: moving_is.item_id(),
                            },
                            adder: Some(client_entity),
                        });

                        held_is.set_quantity(unused_quantity + leftover);

                        // The data entity would have been transferred to the ItemStack now in the inventory
                        if !held_is.is_empty() {
                            held_item_inv.set_itemstack_at(0, Some(held_is), &mut commands);
                        }
                    }
                }
                ClientInventoryMessages::DropOrDepositHeldItemstack => {
                    let Some(mut held_item_inv) = HeldItemStack::get_held_is_inventory_mut(client_entity, &q_children, &mut q_held_item)
                    else {
                        continue;
                    };
                    let Some(mut held_is) = held_item_inv.remove_itemstack_at(0) else {
                        continue;
                    };

                    // TODO: Check if has access to inventory

                    if let Some((inv_ent, mut inventory)) =
                        get_inventory_mut(InventoryIdentifier::Entity(client_entity), &mut q_inventory, &q_structure)
                    {
                        let (leftover, slot) = inventory.insert_itemstack(&held_is, &mut commands);

                        if let Some(slot) = slot {
                            evw_add_item.write(InventoryAddItemEvent {
                                inventory_entity: inv_ent,
                                item: MovedItem {
                                    amount: held_is.quantity() - leftover,
                                    slot: slot as u32,
                                    item_id: held_is.item_id(),
                                },
                                adder: Some(client_entity),
                            });
                        }

                        if leftover != 0 {
                            let Ok((location, g_trans, player_looking, player_velocity)) = q_player.get(client_entity) else {
                                continue;
                            };

                            held_is.set_quantity(leftover);

                            let player_rot = Quat::from_affine3(&g_trans.affine()) * player_looking.rotation;
                            let linvel = player_rot * Vec3::NEG_Z * 4.0 + player_velocity.linvel;

                            let dropped_item_entity = commands
                                .spawn((
                                    PhysicalItem,
                                    *location + linvel.normalize(),
                                    Transform::from_rotation(player_rot),
                                    Velocity {
                                        linvel,
                                        angvel: Vec3::ZERO,
                                    },
                                ))
                                .id();

                            let mut physical_item_inventory = Inventory::new("", 1, None, dropped_item_entity);
                            physical_item_inventory.set_itemstack_at(0, Some(held_is), &mut commands);
                            commands.entity(dropped_item_entity).insert(physical_item_inventory);
                        }
                    }
                }
                ClientInventoryMessages::DepositAndSwapHeldItemstack { inventory_holder, slot } => {
                    let slot = slot as usize;

                    let Some(mut held_item_inv) = HeldItemStack::get_held_is_inventory_mut(client_entity, &q_children, &mut q_held_item)
                    else {
                        continue;
                    };
                    let Some(held_is) = held_item_inv.remove_itemstack_at(0) else {
                        continue;
                    };

                    // TODO: Check if has access to inventory

                    if let Some((inv_ent, mut inventory)) = get_inventory_mut(inventory_holder, &mut q_inventory, &q_structure) {
                        let itemstack_here = inventory.remove_itemstack_at(slot);

                        if let Some(itemstack_here) = itemstack_here.as_ref() {
                            evw_remove_item.write(InventoryRemoveItemEvent {
                                inventory_entity: inv_ent,
                                item: MovedItem {
                                    amount: itemstack_here.quantity(),
                                    slot: slot as u32,
                                    item_id: itemstack_here.item_id(),
                                },
                                remover: Some(client_entity),
                            });
                        }

                        let leftover = inventory.insert_itemstack_at(slot, &held_is, &mut commands);

                        evw_add_item.write(InventoryAddItemEvent {
                            inventory_entity: inv_ent,
                            item: MovedItem {
                                amount: held_is.quantity() - leftover,
                                slot: slot as u32,
                                item_id: held_is.item_id(),
                            },
                            adder: Some(client_entity),
                        });

                        assert_eq!(
                            leftover, 0,
                            "Leftover wasn't 0 somehow? This could only mean something has an invalid stack size"
                        );

                        if let Some(is_here) = itemstack_here {
                            held_item_inv.set_itemstack_at(0, Some(is_here), &mut commands);
                        }
                    }
                }
                ClientInventoryMessages::ThrowItemstack {
                    quantity,
                    slot,
                    inventory_holder,
                } => {
                    let Some((inv_ent, mut inventory)) = get_inventory_mut(inventory_holder, &mut q_inventory, &q_structure) else {
                        continue;
                    };

                    let Some(is) = inventory.itemstack_at(slot as usize) else {
                        continue;
                    };

                    let Ok((location, g_trans, player_looking, player_velocity)) = q_player.get(client_entity) else {
                        continue;
                    };
                    let quantity_being_thrown = quantity.min(is.quantity());

                    evw_remove_item.write(InventoryRemoveItemEvent {
                        inventory_entity: inv_ent,
                        item: MovedItem {
                            amount: quantity_being_thrown,
                            slot,
                            item_id: is.item_id(),
                        },
                        remover: Some(client_entity),
                    });

                    let mut dropped_is = is.clone();
                    dropped_is.set_quantity(quantity_being_thrown);

                    if is.quantity() > quantity_being_thrown {
                        let mut is = is.clone();
                        let qty = is.quantity();
                        is.set_quantity(qty - quantity_being_thrown);
                        inventory.set_itemstack_at(slot as usize, Some(is), &mut commands);
                    } else {
                        inventory.remove_itemstack_at(slot as usize);
                    }

                    let player_rot = Quat::from_affine3(&g_trans.affine()) * player_looking.rotation;
                    let linvel = player_rot * Vec3::NEG_Z * 4.0 + player_velocity.linvel;

                    let dropped_item_entity = commands
                        .spawn((
                            PhysicalItem,
                            *location + linvel.normalize(),
                            Transform::from_rotation(player_rot),
                            Velocity {
                                linvel,
                                angvel: Vec3::ZERO,
                            },
                        ))
                        .id();

                    let mut physical_item_inventory = Inventory::new("", 1, None, dropped_item_entity);
                    physical_item_inventory.set_itemstack_at(0, Some(dropped_is), &mut commands);
                    commands.entity(dropped_item_entity).insert(physical_item_inventory);
                }
                ClientInventoryMessages::ThrowHeldItemstack { quantity } => {
                    let Some(mut held_item_inv) = HeldItemStack::get_held_is_inventory_mut(client_entity, &q_children, &mut q_held_item)
                    else {
                        continue;
                    };
                    let Some(mut held_item_stack) = held_item_inv.remove_itemstack_at(0) else {
                        continue;
                    };

                    if quantity == 0 {
                        continue;
                    }

                    let Ok((location, g_trans, player_looking, player_velocity)) = q_player.get(client_entity) else {
                        continue;
                    };

                    let amount = held_item_stack.quantity().min(quantity);

                    // "Throws" item
                    held_item_stack.decrease_quantity(amount);

                    let mut dropped_is = held_item_stack.clone();
                    dropped_is.set_quantity(amount);

                    let player_rot = player_looking.rotation * Quat::from_affine3(&g_trans.affine());
                    let linvel = player_rot * Vec3::NEG_Z + player_velocity.linvel;

                    let dropped_item_entity = commands
                        .spawn((
                            PhysicalItem,
                            *location + linvel.normalize(),
                            Transform::from_rotation(player_rot),
                            Velocity {
                                linvel,
                                angvel: Vec3::ZERO,
                            },
                        ))
                        .id();

                    let mut physical_item_inventory = Inventory::new("", 1, None, dropped_item_entity);
                    physical_item_inventory.set_itemstack_at(0, Some(dropped_is), &mut commands);
                    commands.entity(dropped_item_entity).insert(physical_item_inventory);

                    if !held_item_stack.is_empty() {
                        held_item_inv.set_itemstack_at(0, Some(held_item_stack), &mut commands);
                    }
                }
                ClientInventoryMessages::InsertHeldItem {
                    quantity,
                    inventory_holder,
                } => {
                    let Some(mut held_item_inv) = HeldItemStack::get_held_is_inventory_mut(client_entity, &q_children, &mut q_held_item)
                    else {
                        continue;
                    };
                    let Some(mut held_item_stack) = held_item_inv.remove_itemstack_at(0) else {
                        continue;
                    };

                    let quantity = held_item_stack.quantity().min(quantity);

                    if let Some((inv_ent, mut inventory)) = get_inventory_mut(inventory_holder, &mut q_inventory, &q_structure) {
                        let unused_leftover = held_item_stack.quantity() - quantity;
                        let mut is = held_item_stack.clone();
                        is.set_quantity(unused_leftover);

                        let (leftover, slot) = inventory.insert_itemstack(&is, &mut commands);

                        if let Some(slot) = slot {
                            evw_add_item.write(InventoryAddItemEvent {
                                inventory_entity: inv_ent,
                                item: MovedItem {
                                    amount: is.quantity() - leftover,
                                    slot: slot as u32,
                                    item_id: is.item_id(),
                                },
                                adder: Some(client_entity),
                            });
                        }

                        held_item_stack.set_quantity(leftover + unused_leftover);

                        if !held_item_stack.is_empty() {
                            held_item_inv.set_itemstack_at(0, Some(held_item_stack), &mut commands);
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        listen_for_inventory_messages
            .in_set(NetworkingSystemsSet::ReceiveMessages) // THIS
            // WAS CHANGED - was between
            .run_if(in_state(GameState::Playing)),
    );
}
