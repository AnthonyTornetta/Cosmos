//! Server-related storage block logic

use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::{
        Block,
        block_events::{BlockEventsSet, BlockInteractEvent},
        data::BlockDataIdentifier,
    },
    entities::player::Player,
    inventory::netty::{InventoryIdentifier, ServerInventoryMessages},
    netty::{NettyChannelServer, cosmos_encoder},
    prelude::StructureBlock,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::Structure,
};

#[derive(Debug, Event, Clone, Copy)]
/// Sent whenever the player opens a storage container
pub struct OpenStorageEvent {
    /// The block the player interacted with to open the storage
    pub block: StructureBlock,
    /// The player's entity
    pub player_ent: Entity,
}

fn handle_block_event(
    mut interact_events: EventReader<BlockInteractEvent>,
    s_query: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    q_player: Query<&Player>,
    mut server: ResMut<RenetServer>,
    mut evw_open_storage: EventWriter<OpenStorageEvent>,
) {
    for ev in interact_events.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        let Ok(structure) = s_query.get(s_block.structure()) else {
            continue;
        };

        let Some(block) = blocks.from_id("cosmos:storage") else {
            continue;
        };

        let block_id = s_block.block_id(structure);

        if block_id == block.id() {
            evw_open_storage.write(OpenStorageEvent {
                block: s_block,
                player_ent: ev.interactor,
            });

            server.send_message(
                player.client_id(),
                NettyChannelServer::Inventory,
                cosmos_encoder::serialize(&ServerInventoryMessages::OpenInventory {
                    owner: InventoryIdentifier::BlockData(BlockDataIdentifier { block: s_block, block_id }),
                }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        handle_block_event
            .in_set(BlockEventsSet::ProcessEvents)
            .run_if(in_state(GameState::Playing)),
    )
    .add_event::<OpenStorageEvent>();
}
