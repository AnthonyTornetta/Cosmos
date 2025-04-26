use bevy::math::Vec3;
use cosmos_core::{
    block::{Block, block_direction::BlockDirection, block_face::BlockFace},
    prelude::ChunkBlockCoordinate,
    registry::Registry,
    structure::{
        ChunkNeighbors,
        block_storage::BlockStorer,
        chunk::{BlockInfo, CHUNK_DIMENSIONS, Chunk},
    },
};

use crate::{
    asset::asset_loading::{BlockNeighbors, BlockTextureIndex, TextureIndex},
    rendering::structure_renderer::{BlockRenderingModes, RenderingMode},
};

pub trait ChunkRendererBackend<C: BlockStorer> {
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

    fn get_texture_index(&self, index: &BlockTextureIndex, neighbors: BlockNeighbors, face: BlockFace, data: BlockInfo) -> TextureIndex;

    fn transform_position(&self, chunk: &C, coords: ChunkBlockCoordinate, direction: BlockDirection, position: Vec3) -> Vec3;
}

pub fn check_block_at<C: BlockStorer>(
    chunk: &C,
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

pub struct ChunkRenderingChecker<'a> {
    pub neighbors: ChunkNeighbors<'a>,
}

impl ChunkRendererBackend<Chunk> for ChunkRenderingChecker<'_> {
    #[inline(always)]
    fn get_texture_index(&self, index: &BlockTextureIndex, neighbors: BlockNeighbors, face: BlockFace, data: BlockInfo) -> TextureIndex {
        index.atlas_index_from_face(face, neighbors, data)
    }

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
            .unwrap_or_else(|_| self.neighbors.check_at(block_coords + delta_chunk_coords))
        else {
            return true;
        };

        check_block_at(chunk, check_coords, blocks, should_connect, block_here, rendering_modes)
    }

    #[inline(always)]
    fn transform_position(&self, _chunk: &Chunk, _coords: ChunkBlockCoordinate, _direction: BlockDirection, position: Vec3) -> Vec3 {
        position
    }
}
