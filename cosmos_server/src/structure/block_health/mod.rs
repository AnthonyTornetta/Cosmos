//! This handles what to do when a block is destroyed

use bevy::prelude::{
    in_state, App, EventReader, EventWriter, IntoSystemConfigs, IntoSystemSetConfigs, Query, Res, ResMut, SystemSet, Update,
};
use bevy_renet2::renet2::RenetServer;
use cosmos_core::{
    block::{block_events::BlockEventsSet, Block},
    events::block_events::BlockChangedEvent,
    netty::{
        cosmos_encoder,
        server_reliable_messages::{BlockHealthUpdate, ServerReliableMessages},
        NettyChannelServer,
    },
    registry::Registry,
    state::GameState,
    structure::{
        block_health::events::{BlockDestroyedEvent, BlockTakeDamageEvent},
        loading::StructureLoadingSet,
        Structure,
    },
};

use super::{planet::biosphere::biosphere_generation::BiosphereGenerationSet, shared::MeltingDownSet};

fn monitor_block_destroyed(
    mut event_reader: EventReader<BlockDestroyedEvent>,
    mut structure_query: Query<&mut Structure>,
    mut event_writer: EventWriter<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
) {
    for ev in event_reader.read() {
        if let Ok(mut structure) = structure_query.get_mut(ev.structure_entity) {
            structure.remove_block_at(ev.block.coords(), &blocks, Some(&mut event_writer));
        }
    }
}

fn monitor_block_health_changed(mut server: ResMut<RenetServer>, mut event_reader: EventReader<BlockTakeDamageEvent>) {
    let changes = event_reader
        .read()
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Handles block health changes
pub enum BlockHealthSet {
    /// Block health changes should be processed (and [`BlockDestroyedEvent`]s sent)
    SendHealthChanges,
    /// Health changes of blocks will be processed (removing blocks with health <= 0)
    ProcessHealthChanges,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            BlockHealthSet::SendHealthChanges,
            BlockHealthSet::ProcessHealthChanges
                .after(BiosphereGenerationSet::GenerateChunkFeatures)
                .after(StructureLoadingSet::StructureLoaded)
                .after(BlockEventsSet::PostProcessEvents)
                .after(MeltingDownSet::ProcessMeltingDown),
        )
            .chain(),
    );

    app.add_systems(
        Update,
        (monitor_block_health_changed, monitor_block_destroyed)
            .in_set(BlockHealthSet::ProcessHealthChanges)
            .in_set(BlockEventsSet::SendEventsForNextFrame)
            .ambiguous_with(BlockEventsSet::SendEventsForNextFrame) // Order of events doesn't matter
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
