//! This handles what to do when a block is destroyed

use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::{Block, block_events::BlockMessagesSet},
    events::block_events::{BlockChangedMessage, BlockChangedReason},
    netty::{
        NettyChannelServer, cosmos_encoder,
        server_reliable_messages::{BlockHealthUpdate, ServerReliableMessages},
    },
    registry::Registry,
    state::GameState,
    structure::{
        Structure,
        block_health::events::{BlockDestroyedMessage, BlockTakeDamageMessage},
        loading::StructureLoadingSet,
    },
};

use super::{planet::biosphere::biosphere_generation::BiosphereGenerationSet, shared::MeltingDownSet};

fn monitor_block_destroyed(
    mut event_reader: MessageReader<BlockDestroyedMessage>,
    mut structure_query: Query<&mut Structure>,
    mut event_writer: MessageWriter<BlockChangedMessage>,
    blocks: Res<Registry<Block>>,
) {
    for ev in event_reader.read() {
        if let Ok(mut structure) = structure_query.get_mut(ev.structure_entity) {
            structure.remove_block_at(
                ev.block.coords(),
                &blocks,
                Some((&mut event_writer, BlockChangedReason::TookDamage { causer: ev.causer })),
            );
        }
    }
}

fn monitor_block_health_changed(mut server: ResMut<RenetServer>, mut event_reader: MessageReader<BlockTakeDamageMessage>) {
    let changes = event_reader
        .read()
        .map(|ev| BlockHealthUpdate {
            block: ev.block,
            new_health: ev.new_health,
            structure_entity: ev.structure_entity,
            causer: ev.causer,
        })
        .collect::<Vec<BlockHealthUpdate>>();

    if !changes.is_empty() {
        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::BlockHealthChange { changes }),
        );
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Handles block health changes
pub enum BlockHealthSet {
    /// Block health changes should be processed (and [`BlockDestroyedMessage`]s sent)
    SendHealthChanges,
    /// Health changes of blocks will be processed (removing blocks with health <= 0)
    ProcessHealthChanges,
    /// Recieves [`BlockDestroyedMessage`] messages and removes the blocks
    RemoveBlocks,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        FixedUpdate,
        (
            BlockHealthSet::SendHealthChanges,
            BlockHealthSet::ProcessHealthChanges
                .after(BiosphereGenerationSet::GenerateChunkFeatures)
                .after(StructureLoadingSet::StructureLoaded)
                .after(BlockMessagesSet::PostProcessMessages)
                .after(MeltingDownSet::ProcessMeltingDown),
        )
            .chain(),
    );

    app.add_systems(
        FixedUpdate,
        (monitor_block_health_changed, monitor_block_destroyed)
            .in_set(BlockHealthSet::RemoveBlocks)
            .in_set(BlockMessagesSet::SendMessagesForNextFrame)
            .ambiguous_with(BlockMessagesSet::SendMessagesForNextFrame) // Order of events doesn't matter
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
