use cosmos_core::{
    block::{block_direction::BlockDirection, block_face::BlockFace, Block},
    prelude::ChunkBlockCoordinate,
    registry::Registry,
    structure::{chunk::CHUNK_DIMENSIONS, lod_chunk::LodChunk},
};

use crate::{
    asset::asset_loading::{BlockNeighbors, BlockTextureIndex, TextureIndex},
    rendering::structure_renderer::BlockRenderingModes,
};

use super::neighbor_checking::{check_block_at, ChunkRendererBackend};

pub struct LodChunkRenderingChecker<'a> {
    pub scale: f32,
    pub neg_x: Option<&'a LodChunk>,
    pub pos_x: Option<&'a LodChunk>,
    pub neg_y: Option<&'a LodChunk>,
    pub pos_y: Option<&'a LodChunk>,
    pub neg_z: Option<&'a LodChunk>,
    pub pos_z: Option<&'a LodChunk>,
}

impl<'a> ChunkRendererBackend<LodChunk> for LodChunkRenderingChecker<'a> {
    #[inline(always)]
    fn get_texture_index(&self, index: &BlockTextureIndex, neighbors: BlockNeighbors, face: BlockFace) -> Option<TextureIndex> {
        let maybe_img_idx = if self.scale > 8.0 {
            index
                .atlas_index_for_lod(neighbors)
                .map(Some)
                .unwrap_or_else(|| index.atlas_index_from_face(face, neighbors))
        } else {
            index.atlas_index_from_face(face, neighbors)
        };
        maybe_img_idx
    }

    fn check_should_render(
        &self,
        c: &LodChunk,
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
}
