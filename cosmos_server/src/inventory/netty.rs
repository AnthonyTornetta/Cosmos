//! Syncs player inventories

use bevy::{
    ecs::world::Mut,
    log::warn,
    math::{Quat, Vec3},
    prelude::{
        in_state, App, Changed, Commands, Entity, GlobalTransform, IntoSystemConfigs, Query, RemovedComponents, Res, ResMut, Update,
    },
};
use bevy_rapier3d::prelude::Velocity;
use bevy_renet2::renet2::RenetServer;
use cosmos_core::{
    ecs::bundles::BundleStartingRotation,
    entities::player::Player,
    inventory::{
        netty::{ClientInventoryMessages, InventoryIdentifier, ServerInventoryMessages},
        HeldItemStack, Inventory,
    },
    item::physical_item::PhysicalItem,
    netty::{cosmos_encoder, server::ServerLobby, NettyChannelClient, NettyChannelServer},
    persistence::LoadingDistance,
    physics::location::Location,
    state::GameState,
    structure::Structure,
};

use crate::entities::player::PlayerLooking;

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

    for removed_held_item in removed_held_itemstacks.read() {
        if let Ok(player) = player_query.get(removed_held_item) {
            server.send_message(
                player.id(),
                NettyChannelServer::Inventory,
                cosmos_encoder::serialize(&ServerInventoryMessages::HeldItemstack { itemstack: None }),
            );
        }
    }
}

fn get_inventory_mut<'a>(
    identifier: InventoryIdentifier,
    q_inventory: &'a mut Query<&mut Inventory>,
    q_structure: &'a Query<&Structure>,
) -> Option<Mut<'a, Inventory>> {
    match identifier {
        InventoryIdentifier::Entity(entity) => q_inventory.get_mut(entity).ok(),
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

            q_inventory.get_mut(block_data_ent).ok()
        }
    }
}

fn get_many_inventories_mut<'a, const N: usize>(
    identifiers: [InventoryIdentifier; N],
    q_inventory: &'a mut Query<&mut Inventory>,
    q_structure: &'a Query<&Structure>,
) -> Option<[Mut<'a, Inventory>; N]> {
    let ents = identifiers
        .into_iter()
        .map(|x| match x {
            InventoryIdentifier::Entity(entity) => Some(entity),
            InventoryIdentifier::BlockData(block_data) => {
                let structure = q_structure.get(block_data.block.structure()).ok()?;

                structure.block_data(block_data.block.coords())
            }
        })
        .collect::<Option<Vec<Entity>>>()?;

    let ents = ents.try_into().expect("This is guarenteed to be the same size as input");

    q_inventory.get_many_mut(ents).ok()
}

fn listen_for_inventory_messages(
    mut commands: Commands,
    mut q_inventory: Query<&mut Inventory>,
    q_structure: Query<&Structure>,
    mut held_item_query: Query<&mut HeldItemStack>,
    mut server: ResMut<RenetServer>,
    q_player: Query<(&Location, &GlobalTransform, &PlayerLooking, &Velocity)>,
    lobby: Res<ServerLobby>,
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
                        if let Some(mut inventory) = get_inventory_mut(inventory_a, &mut q_inventory, &q_structure) {
                            inventory
                                .self_swap_slots(slot_a as usize, slot_b as usize, &mut commands)
                                .unwrap_or_else(|_| panic!("Got bad inventory slots from player! {}, {}", slot_a, slot_b));
                        }
                    } else if let Some([mut inventory_a, mut inventory_b]) =
                        get_many_inventories_mut([inventory_a, inventory_b], &mut q_inventory, &q_structure)
                    {
                        inventory_a
                            .swap_slots(slot_a as usize, &mut inventory_b, slot_b as usize, &mut commands)
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
                        if let Some(mut inventory) = get_inventory_mut(from_inventory, &mut q_inventory, &q_structure) {
                            inventory
                                .auto_move(from_slot as usize, quantity, &mut commands)
                                .unwrap_or_else(|_| panic!("Got bad inventory slot from player! {}", from_slot));
                        }
                    } else if let Some([mut from_inventory, mut to_inventory]) =
                        get_many_inventories_mut([from_inventory, to_inventory], &mut q_inventory, &q_structure)
                    {
                        let from_slot = from_slot as usize;
                        if let Some(mut is) = from_inventory.remove_itemstack_at(from_slot) {
                            let (leftover, _) = to_inventory.insert_itemstack(&is, &mut commands);
                            if leftover == 0 {
                                from_inventory.remove_itemstack_at(from_slot);
                            } else if leftover == is.quantity() {
                                from_inventory.set_itemstack_at(from_slot, Some(is), &mut commands);

                                from_inventory
                                    .auto_move(from_slot, quantity, &mut commands)
                                    .unwrap_or_else(|_| panic!("Got bad inventory slot from player! {}", from_slot));
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
                        if let Some(mut inventory) = get_inventory_mut(from_inventory, &mut q_inventory, &q_structure) {
                            inventory
                                .self_move_itemstack(from_slot as usize, to_slot as usize, quantity, &mut commands)
                                .unwrap_or_else(|_| panic!("Got bad inventory slots from player! {}, {}", from_slot, to_slot));
                        }
                    } else if let Some([mut inventory_a, mut inventory_b]) =
                        get_many_inventories_mut([from_inventory, to_inventory], &mut q_inventory, &q_structure)
                    {
                        inventory_a
                            .move_itemstack(from_slot as usize, &mut inventory_b, to_slot as usize, quantity, &mut commands)
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

                    if let Some(mut inventory) = get_inventory_mut(inventory_holder, &mut q_inventory, &q_structure) {
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

                    if let Some(mut inventory) = get_inventory_mut(inventory_holder, &mut q_inventory, &q_structure) {
                        let quantity = quantity.min(held_is.quantity()); // make sure we don't deposit more than we have
                        let mut moving_is = held_is.clone();
                        moving_is.set_quantity(quantity);

                        let unused_quantity = held_is.quantity() - quantity;

                        let leftover = inventory.insert_itemstack_at(slot, &moving_is, &mut commands);

                        held_is.set_quantity(unused_quantity + leftover);

                        // The data entity would have been transferred to the ItemStack now in the inventory
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

                    if let Some(mut inventory) = get_inventory_mut(inventory_holder, &mut q_inventory, &q_structure) {
                        let itemstack_here = inventory.remove_itemstack_at(slot);

                        let leftover = inventory.insert_itemstack_at(slot, &held_item_stack, &mut commands);

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
                ClientInventoryMessages::ThrowItemstack {
                    quantity,
                    slot,
                    inventory_holder,
                } => {
                    let Some(mut inventory) = get_inventory_mut(inventory_holder, &mut q_inventory, &q_structure) else {
                        continue;
                    };

                    let Some(is) = inventory.itemstack_at(slot as usize) else {
                        continue;
                    };

                    let Ok((location, g_trans, player_looking, player_velocity)) = q_player.get(client_entity) else {
                        continue;
                    };
                    let quantity_being_thrown = quantity.min(is.quantity());

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
                            LoadingDistance::new(1, 2),
                            BundleStartingRotation(player_rot),
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
                    let Ok(mut held_item_stack) = held_item_query.get_mut(client_entity) else {
                        // Perhaps the client needs updated
                        server.send_message(
                            client_id,
                            NettyChannelServer::Inventory,
                            cosmos_encoder::serialize(&ServerInventoryMessages::HeldItemstack { itemstack: None }),
                        );
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

                    let mut dropped_is = held_item_stack.0.clone();
                    dropped_is.set_quantity(amount);

                    let player_rot = player_looking.rotation * Quat::from_affine3(&g_trans.affine());
                    let linvel = player_rot * Vec3::NEG_Z + player_velocity.linvel;

                    let dropped_item_entity = commands
                        .spawn((
                            PhysicalItem,
                            *location + linvel.normalize(),
                            LoadingDistance::new(1, 2),
                            BundleStartingRotation(player_rot),
                            Velocity {
                                linvel,
                                angvel: Vec3::ZERO,
                            },
                        ))
                        .id();

                    let mut physical_item_inventory = Inventory::new("", 1, None, dropped_item_entity);
                    physical_item_inventory.set_itemstack_at(0, Some(dropped_is), &mut commands);
                    commands.entity(dropped_item_entity).insert(physical_item_inventory);

                    if held_item_stack.is_empty() {
                        // no longer remove since it's thrown, right?
                        // held_item_stack.remove(&mut commands);
                        commands.entity(client_entity).remove::<HeldItemStack>();
                    }
                }
                ClientInventoryMessages::InsertHeldItem {
                    quantity,
                    inventory_holder,
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

                    if let Some(mut inventory) = get_inventory_mut(inventory_holder, &mut q_inventory, &q_structure) {
                        let unused_leftover = held_item_stack.quantity() - quantity;
                        let mut is = held_item_stack.clone();
                        is.set_quantity(unused_leftover);

                        let (leftover, _) = inventory.insert_itemstack(&is, &mut commands);

                        held_item_stack.set_quantity(leftover + unused_leftover);

                        // Data would have been transferred, so no need to remove.
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
        (listen_for_inventory_messages, sync_held_items)
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
