//! Syncs player inventories

use bevy::prelude::{in_state, App, Changed, Entity, IntoSystemConfigs, Query, ResMut, Update};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    inventory::{netty::ServerInventoryMessages, Inventory},
    netty::{cosmos_encoder, NettyChannelServer},
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

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, sync.run_if(in_state(GameState::Playing)));
}
