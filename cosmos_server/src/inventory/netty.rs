//! Syncs player inventories

use bevy::prelude::{in_state, App, Changed, Entity, IntoSystemConfigs, Query, ResMut, Update};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    inventory::{
        netty::{ClientInventoryMessages, ServerInventoryMessages},
        Inventory,
    },
    netty::{cosmos_encoder, NettyChannelClient, NettyChannelServer},
};

use crate::state::GameState;

fn sync(query: Query<(Entity, &Inventory), Changed<Inventory>>, mut server: ResMut<RenetServer>) {
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

fn listen(mut query: Query<&mut Inventory>, mut server: ResMut<RenetServer>) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::Inventory) {
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
                        if let Ok(mut inventory) = query.get_mut(inventory_a) {
                            inventory
                                .self_swap_slots(slot_a as usize, slot_b as usize)
                                .expect(format!("Got bad inventory slots from player! {}, {}", slot_a, slot_b).as_str());
                        }
                    } else {
                        if let Ok([mut inventory_a, mut inventory_b]) = query.get_many_mut([inventory_a, inventory_b]) {
                            inventory_a
                                .swap_slots(slot_a as usize, &mut inventory_b, slot_b as usize)
                                .expect(format!("Got bad inventory slots from player! {}, {}", slot_a, slot_b).as_str());
                        }
                    }
                }
                ClientInventoryMessages::AutoMove {
                    from_slot,
                    from_inventory,
                    to_inventory,
                } => {
                    if from_inventory == to_inventory {
                        if let Ok(mut inventory) = query.get_mut(from_inventory) {
                            inventory
                                .auto_move(from_slot)
                                .expect(format!("Got bad inventory slot from player! {}", from_slot).as_str());
                        }
                    } else {
                        panic!("Not implemented yet!");
                    }
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (listen, sync).run_if(in_state(GameState::Playing)));
}
