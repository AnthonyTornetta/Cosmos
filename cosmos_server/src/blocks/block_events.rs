use bevy::{prelude::*, utils::HashMap};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    events::block_events::BlockChangedEvent,
    netty::{
        cosmos_encoder,
        server_reliable_messages::{BlockChanged, BlocksChangedPacket, ServerReliableMessages},
        NettyChannelServer,
    },
};

use crate::state::GameState;

fn handle_block_changed_event(mut event_reader: EventReader<BlockChangedEvent>, mut server: ResMut<RenetServer>) {
    let iter_len = event_reader.read().len();
    let mut map = HashMap::new();
    for ev in event_reader.read() {
        if !map.contains_key(&ev.structure_entity) {
            map.insert(ev.structure_entity, Vec::with_capacity(iter_len));
        }
        map.get_mut(&ev.structure_entity).expect("Set above").push(BlockChanged {
            coordinates: ev.block,
            block_id: ev.new_block,
            block_up: ev.new_block_up,
        });
    }

    for (entity, v) in map {
        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::BlockChange {
                structure_entity: entity,
                blocks_changed_packet: BlocksChangedPacket(v),
            }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, handle_block_changed_event.run_if(in_state(GameState::Playing)));
}
