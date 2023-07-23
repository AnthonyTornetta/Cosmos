//! Some useful utilities for generating terrain

use bevy::prelude::EventWriter;
use cosmos_core::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    registry::Registry,
    structure::{
        coordinates::{BlockCoordinate, UnboundBlockCoordinate},
        rotate, Structure,
    },
};

/// Sets the given block with the given relative rotation at the correct offsets, taking planet face into account.
pub(crate) fn fill(
    origin: BlockCoordinate,
    offsets: &[UnboundBlockCoordinate],
    block: &Block,
    block_up: BlockFace,
    planet_face: BlockFace,
    structure: &mut Structure,
    blocks: &Registry<Block>,
    event_writer: &mut EventWriter<BlockChangedEvent>,
) {
    for offset in offsets {
        if let Ok(rotated_block_pos) = rotate(
            origin,
            *offset,
            (structure.blocks_width(), structure.blocks_height(), structure.blocks_length()),
            planet_face,
        ) {
            structure.set_block_at(
                rotated_block_pos,
                block,
                BlockFace::rotate_face(block_up, planet_face),
                blocks,
                Some(event_writer),
            );
        }
    }
}
