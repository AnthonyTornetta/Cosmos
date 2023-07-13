//! Some useful utilities for generating terrain

use bevy::prelude::{
    App, Commands, Component, Entity, EventReader, EventWriter, IntoSystemAppConfig, IntoSystemConfigs, OnEnter, OnUpdate, Query, Res,
};
use cosmos_core::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::Registry,
    structure::{chunk::CHUNK_DIMENSIONS, planet::Planet, rotate, ChunkInitEvent, Structure},
    utils::resource_wrapper::ResourceWrapper,
};

/// Sets the given block with the given relative rotation at the correct offsets, taking planet face into account.
pub(crate) fn fill(
    origin: (usize, usize, usize),
    offsets: &[(i32, i32, i32)],
    block: &Block,
    block_up: BlockFace,
    planet_face: BlockFace,
    structure: &mut Structure,
    blocks: &Registry<Block>,
    event_writer: &mut EventWriter<BlockChangedEvent>,
) {
    for offset in offsets {
        structure.set_block_at_tuple(
            rotate(origin, *offset, planet_face),
            block,
            BlockFace::rotate_face(block_up, planet_face),
            blocks,
            Some(event_writer),
        );
    }
}
