//! This handles what to do when a block is destroyed

use bevy::prelude::{in_state, App, EventReader, EventWriter, IntoSystemConfigs, Query, Res, ResMut, Update};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::Block,
    events::block_events::BlockChangedEvent,
    netty::{
        cosmos_encoder,
        server_reliable_messages::{BlockHealthUpdate, ServerReliableMessages},
        NettyChannelServer,
    },
    registry::Registry,
    structure::{
        block_health::events::{BlockDestroyedEvent, BlockTakeDamageEvent},
        Structure,
    },
};

use crate::state::GameState;

fn monitor_block_destroyed(
    mut event_reader: EventReader<BlockDestroyedEvent>,
    mut structure_query: Query<&mut Structure>,
    mut event_writer: EventWriter<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
) {
    for ev in event_reader.iter() {
        if let Ok(mut structure) = structure_query.get_mut(ev.structure_entity) {
            structure.remove_block_at(ev.block.coords(), &blocks, Some(&mut event_writer));
        }
    }
}

fn monitor_block_health_changed(mut server: ResMut<RenetServer>, mut event_reader: EventReader<BlockTakeDamageEvent>) {
    let changes = event_reader
        .iter()
        .map(|ev| BlockHealthUpdate {
            block: ev.block,
            new_health: ev.new_health,
            structure_entity: ev.structure_entity,
        })
        .collect::<Vec<BlockHealthUpdate>>();

    if !changes.is_empty() {
        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::BlockHealthChange { changes }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (monitor_block_destroyed, monitor_block_health_changed).run_if(in_state(GameState::Playing)),
    );
}
