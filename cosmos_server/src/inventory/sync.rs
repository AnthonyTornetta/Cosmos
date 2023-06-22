//! Syncs player inventories

use bevy::prelude::{App, Changed, Entity, IntoSystemConfig, OnUpdate, Query, ResMut};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    inventory::Inventory,
    netty::{cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannelServer},
};

use crate::state::GameState;

fn sync(query: Query<(Entity, &Inventory), Changed<Inventory>>, mut server: ResMut<RenetServer>) {
    for (entity, inventory) in query.iter() {
        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::EntityInventory {
                serialized_inventory: cosmos_encoder::serialize(&inventory),
                owner: entity,
            }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(sync.in_set(OnUpdate(GameState::Playing)));
}
