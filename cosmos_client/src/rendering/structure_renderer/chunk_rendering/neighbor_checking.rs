use bevy::math::Vec3;
use cosmos_core::{
    block::{Block, block_direction::BlockDirection, block_face::BlockFace},
    prelude::ChunkBlockCoordinate,
    registry::Registry,
    structure::{
        block_storage::BlockStorer,
        chunk::{CHUNK_DIMENSIONS, Chunk},
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

    fn get_texture_index(&self, index: &BlockTextureIndex, neighbors: BlockNeighbors, face: BlockFace) -> Option<TextureIndex>;

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
    pub neg_x: Option<&'a Chunk>,
    pub pos_x: Option<&'a Chunk>,
    pub neg_y: Option<&'a Chunk>,
    pub pos_y: Option<&'a Chunk>,
    pub neg_z: Option<&'a Chunk>,
    pub pos_z: Option<&'a Chunk>,
}

impl ChunkRendererBackend<Chunk> for ChunkRenderingChecker<'_> {
    #[inline(always)]
    fn get_texture_index(&self, index: &BlockTextureIndex, neighbors: BlockNeighbors, face: BlockFace) -> Option<TextureIndex> {
        index.atlas_index_from_face(face, neighbors)
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
            .unwrap_or_else(|_| match BlockDirection::from_chunk_block_coordinates(delta_chunk_coords) {
                BlockDirection::NegX => self
                    .neg_x
                    .map(|neg_x_chunk| {
                        Some((
                            neg_x_chunk,
                            ChunkBlockCoordinate::new(CHUNK_DIMENSIONS - 1, block_coords.y, block_coords.z).expect("Invalid coordinate"),
                        ))
                    })
                    .unwrap_or(None),
                BlockDirection::PosX => self
                    .pos_x
                    .map(|pos_x_chunk| {
                        Some((
                            pos_x_chunk,
                            ChunkBlockCoordinate::new(0, block_coords.y, block_coords.z).expect("Invalid coordinate"),
                        ))
                    })
                    .unwrap_or(None),
                BlockDirection::NegY => self
                    .neg_y
                    .map(|neg_y_chunk| {
                        Some((
                            neg_y_chunk,
                            ChunkBlockCoordinate::new(block_coords.x, CHUNK_DIMENSIONS - 1, block_coords.z).expect("Invalid coordinate"),
                        ))
                    })
                    .unwrap_or(None),
                BlockDirection::PosY => self
                    .pos_y
                    .map(|pos_y_chunk| {
                        Some((
                            pos_y_chunk,
                            ChunkBlockCoordinate::new(block_coords.x, 0, block_coords.z).expect("Invalid coordinate"),
                        ))
                    })
                    .unwrap_or(None),
                BlockDirection::NegZ => self
                    .neg_z
                    .map(|neg_z_chunk| {
                        Some((
                            neg_z_chunk,
                            ChunkBlockCoordinate::new(block_coords.x, block_coords.y, CHUNK_DIMENSIONS - 1).expect("Invalid coordinate"),
                        ))
                    })
                    .unwrap_or(None),
                BlockDirection::PosZ => self
                    .pos_z
                    .map(|pos_z_chunk| {
                        Some((
                            pos_z_chunk,
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

    #[inline(always)]
    fn transform_position(&self, _chunk: &Chunk, _coords: ChunkBlockCoordinate, _direction: BlockDirection, position: Vec3) -> Vec3 {
        position
    }
}
