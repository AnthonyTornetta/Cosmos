use bevy::{prelude::*, utils::HashMap};
use bevy_renet2::renet2::RenetServer;
use cosmos_core::{
    events::block_events::{BlockChangedEvent, BlockDataChangedEvent},
    netty::{
        cosmos_encoder,
        server_reliable_messages::{BlockChanged, BlocksChangedPacket, ServerReliableMessages},
        system_sets::NetworkingSystemsSet,
        NettyChannelServer,
    },
    prelude::Structure,
    state::GameState,
};

use crate::structure::block_health::BlockHealthSet;

fn handle_block_changed_event(
    mut evr_block_changed_event: EventReader<BlockChangedEvent>,
    mut evr_block_data_changed: EventReader<BlockDataChangedEvent>,
    mut server: ResMut<RenetServer>,
    q_structure: Query<&Structure>,
) {
    let mut events_iter = evr_block_changed_event.read();
    let iter_len = events_iter.len();
    let mut map = HashMap::new();

    for ev in events_iter {
        if !map.contains_key(&ev.block.structure()) {
            map.insert(ev.block.structure(), Vec::with_capacity(iter_len));
        }
        map.get_mut(&ev.block.structure()).expect("Set above").push(BlockChanged {
            coordinates: ev.block,
            block_id: ev.new_block,
            block_info: ev.new_block_info,
        });
    }
    for ev in evr_block_data_changed.read() {
        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            continue;
        };
        if !map.contains_key(&ev.block.structure()) {
            map.insert(ev.block.structure(), Vec::with_capacity(iter_len));
        }
        map.get_mut(&ev.block.structure()).expect("Set above").push(BlockChanged {
            coordinates: ev.block,
            block_id: structure.block_id_at(ev.block.coords()),
            block_info: structure.block_info_at(ev.block.coords()),
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
    app.add_systems(
        Update,
        handle_block_changed_event
            .in_set(NetworkingSystemsSet::SyncComponents)
            .after(BlockHealthSet::ProcessHealthChanges)
            .run_if(in_state(GameState::Playing)),
    );
}
