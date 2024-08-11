use cosmos_core::{
    block::{block_direction::BlockDirection, Block},
    prelude::ChunkBlockCoordinate,
    registry::Registry,
    structure::{
        block_storage::BlockStorer,
        chunk::{Chunk, CHUNK_DIMENSIONS},
    },
};

use crate::rendering::structure_renderer::{BlockRenderingModes, RenderingMode};

pub trait RenderingChecker<C: BlockStorer> {
    fn check_should_render(
        &self,
        c: &C,
        block_here: &Block,
        block_coords: ChunkBlockCoordinate,
        blocks: &Registry<Block>,
        direction_to_check: BlockDirection,
        should_connect: &mut bool,
        rendering_modes: &BlockRenderingModes,
    ) -> bool;
}

pub struct ChunkRenderingChecker<'a> {
    pub neg_x: Option<&'a Chunk>,
    pub pos_x: Option<&'a Chunk>,
    pub neg_y: Option<&'a Chunk>,
    pub pos_y: Option<&'a Chunk>,
    pub neg_z: Option<&'a Chunk>,
    pub pos_z: Option<&'a Chunk>,
}

impl<'a> RenderingChecker<Chunk> for ChunkRenderingChecker<'a> {
    fn check_should_render(
        &self,
        c: &Chunk,
        block_here: &Block,
        block_coords: ChunkBlockCoordinate,
        blocks: &Registry<Block>,
        direction_to_check: BlockDirection,
        should_connect: &mut bool,
        rendering_modes: &BlockRenderingModes,
    ) -> bool {
        let delta_chunk_coords = direction_to_check.to_chunk_block_coordinates();

        let Some((chunk, check_coords)) = ChunkBlockCoordinate::try_from(block_coords + delta_chunk_coords)
            .map(|coord| Some((c, coord)))
            .unwrap_or_else(|_| match BlockDirection::from_chunk_block_coordinates(delta_chunk_coords) {
                BlockDirection::NegX => self
                    .neg_x
                    .map(|left_chunk| {
                        Some((
                            left_chunk,
                            ChunkBlockCoordinate::new(CHUNK_DIMENSIONS - 1, block_coords.y, block_coords.z).expect("Invalid coordinate"),
                        ))
                    })
                    .unwrap_or(None),
                BlockDirection::PosX => self
                    .pos_x
                    .map(|right_chunk| {
                        Some((
                            right_chunk,
                            ChunkBlockCoordinate::new(0, block_coords.y, block_coords.z).expect("Invalid coordinate"),
                        ))
                    })
                    .unwrap_or(None),
                BlockDirection::NegY => self
                    .neg_y
                    .map(|bottom_chunk| {
                        Some((
                            bottom_chunk,
                            ChunkBlockCoordinate::new(block_coords.x, CHUNK_DIMENSIONS - 1, block_coords.z).expect("Invalid coordinate"),
                        ))
                    })
                    .unwrap_or(None),
                BlockDirection::PosY => self
                    .pos_y
                    .map(|top_chunk| {
                        Some((
                            top_chunk,
                            ChunkBlockCoordinate::new(block_coords.x, 0, block_coords.z).expect("Invalid coordinate"),
                        ))
                    })
                    .unwrap_or(None),
                BlockDirection::NegZ => self
                    .neg_z
                    .map(|front_chunk| {
                        Some((
                            front_chunk,
                            ChunkBlockCoordinate::new(block_coords.x, block_coords.y, CHUNK_DIMENSIONS - 1).expect("Invalid coordinate"),
                        ))
                    })
                    .unwrap_or(None),
                BlockDirection::PosZ => self
                    .pos_z
                    .map(|back_chunk| {
                        Some((
                            back_chunk,
                            ChunkBlockCoordinate::new(block_coords.x, block_coords.y, 0).expect("Invalid coordinate"),
                        ))
                    })
                    .unwrap_or(None),
            })
        else {
            return true;
        };

        check_block_at(chunk, check_coords, blocks, should_connect, block_here, rendering_modes)
    }
}

fn check_block_at(
    chunk: &Chunk,
    check_coords: ChunkBlockCoordinate,
    blocks: &Registry<Block>,
    should_connect: &mut bool,
    actual_block: &Block,
    rendering_modes: &BlockRenderingModes,
) -> bool {
    let block_id_checking = chunk.block_at(check_coords);
    let block_checking = blocks.from_numeric_id(block_id_checking);
    *should_connect = actual_block.should_connect_with(block_checking);

    let custom_rendered = rendering_modes.rendering_mode(block_id_checking);

    // A block adjacent is custom
    custom_rendered == RenderingMode::Custom
        || (!(actual_block.is_fluid() && block_checking == actual_block) && (block_checking.is_see_through() || !actual_block.is_full()))
}
