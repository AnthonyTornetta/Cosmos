//! Syncs the inventories with the server-provided inventories

use bevy::prelude::{in_state, warn, App, Commands, Entity, IntoSystemConfigs, Query, Res, ResMut, Update};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    ecs::NeedsDespawned,
    inventory::netty::ServerInventoryMessages,
    netty::{cosmos_encoder, NettyChannelServer},
};

use crate::{netty::mapping::NetworkMapping, state::game_state::GameState};

use super::HeldItemStack;

fn sync(
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
    mut commands: Commands,
    mut held_item_query: Query<(Entity, &mut HeldItemStack)>,
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
                    warn!("Error: unrecognized entity {} received from server!", owner.index());
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
