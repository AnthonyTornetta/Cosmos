use bevy::prelude::{App, Changed, Entity, IntoSystemConfig, OnUpdate, Query, ResMut};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    inventory::Inventory,
    netty::{server_reliable_messages::ServerReliableMessages, NettyChannel},
};

use crate::state::GameState;

fn sync(query: Query<(Entity, &Inventory), Changed<Inventory>>, mut server: ResMut<RenetServer>) {
    for (entity, inventory) in query.iter() {
        server.broadcast_message(
            NettyChannel::Reliable.id(),
            bincode::serialize(&ServerReliableMessages::EntityInventory {
                serialized_inventory: bincode::serialize(&inventory).unwrap(),
                owner: entity,
            })
            .unwrap(),
        );
    }
}

pub fn register(app: &mut App) {
    app.add_system(sync.in_set(OnUpdate(GameState::Playing)));
}
