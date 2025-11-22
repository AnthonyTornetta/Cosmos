//! Some useful utilities for generating terrain

use bevy::prelude::MessageWriter;
use cosmos_core::{
    block::{
        Block,
        block_face::BlockFace,
        block_rotation::{BlockRotation, BlockSubRotation},
    },
    events::block_events::BlockChangedMessage,
    registry::Registry,
    structure::{
        Structure,
        coordinates::{BlockCoordinate, UnboundBlockCoordinate},
        rotate,
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
    event_writer: &mut MessageWriter<BlockChangedMessage>,
) {
    for offset in offsets {
        if let Ok(rotated_block_pos) = rotate(origin, *offset, structure.block_dimensions(), planet_face) {
            structure.set_block_at(
                rotated_block_pos,
                block,
                BlockRotation::new(block_up, BlockSubRotation::None).combine(BlockRotation::new(planet_face, BlockSubRotation::None)),
                blocks,
                Some(event_writer),
            );
        }
    }
}
