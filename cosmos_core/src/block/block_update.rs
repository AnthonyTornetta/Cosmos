//! Messages sent to adjacent blocks on block changes

use bevy::prelude::*;

use crate::{
    ecs::mut_events::{MutMessage, MutMessagesCommand},
    events::block_events::BlockChangedMessage,
    structure::{Structure, coordinates::BlockCoordinate, structure_block::StructureBlock},
};

use super::{block_events::BlockMessagesSet, block_face::ALL_BLOCK_FACES};

#[derive(Debug, Clone, Copy, Message, PartialEq, Eq)]
/// This event is sent whenever an adjacent block is changed
pub struct BlockUpdate {
    block: StructureBlock,
    cancelled: bool,
}

impl BlockUpdate {
    /// Creates a new block update
    pub fn new(block: StructureBlock) -> Self {
        Self { block, cancelled: false }
    }

    /// The structure that was updated
    pub fn structure_entity(&self) -> Entity {
        self.block.structure()
    }

    /// The block that was changed
    pub fn block(&self) -> StructureBlock {
        self.block
    }

    /// If the event has been cancelled
    pub fn cancelled(&self) -> bool {
        self.cancelled
    }

    /// Cancels the event (will do nothing)
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Sets if the event has been cancelled or not
    pub fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

/// Sends block updates when blocks are changed
pub fn send_block_updates(
    structure_query: Query<&Structure>,
    mut block_chage_event: MessageReader<BlockChangedMessage>,
    mut event_writer: MessageWriter<MutMessage<BlockUpdate>>,
) {
    let block_updates = block_chage_event
        .read()
        .filter_map(|ev| {
            let Ok(structure) = structure_query.get(ev.block.structure()) else {
                return None;
            };

            Some(ALL_BLOCK_FACES.iter().filter_map(|face| {
                let coord = face.direction().to_coordinates() + ev.block.coords();
                let Ok(coord) = BlockCoordinate::try_from(coord) else {
                    return None;
                };
                if !structure.is_within_blocks(coord) {
                    return None;
                }

                Some(MutMessage::from(BlockUpdate {
                    block: StructureBlock::new(coord, ev.block.structure()),
                    cancelled: false,
                }))
            }))
        })
        .flatten();

    event_writer.write_batch(block_updates);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, send_block_updates.in_set(BlockMessagesSet::SendBlockUpdateMessages))
        .add_mut_event::<BlockUpdate>();
}
