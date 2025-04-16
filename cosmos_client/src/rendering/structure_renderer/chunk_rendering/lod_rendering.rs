use bevy::math::Vec3;
use cosmos_core::{
    block::{Block, block_direction::BlockDirection, block_face::BlockFace, blocks::AIR_BLOCK_ID},
    prelude::{BlockCoordinate, ChunkBlockCoordinate, UnboundBlockCoordinate},
    registry::{Registry, identifiable::Identifiable},
    structure::{
        block_storage::BlockStorer,
        coordinates::{CoordinateType, UnboundCoordinateType},
        lod::Lod,
        lod_chunk::{LodBlockSubScale, LodChunk},
    },
};

use crate::{
    asset::asset_loading::{BlockNeighbors, BlockTextureIndex, TextureIndex},
    rendering::structure_renderer::BlockRenderingModes,
};

use super::neighbor_checking::ChunkRendererBackend;

pub struct LodChunkRenderingChecker<'a> {
    pub scale: CoordinateType,
    pub lod_root: &'a Lod,
    pub negative_most_coord: BlockCoordinate,
    // How big is the LOD root (in block coordinates)
    pub lod_root_scale: CoordinateType,
}

impl LodChunkRenderingChecker<'_> {
    fn inner_check_should_render(
        &self,
        chunk: &LodChunk,
        chunk_block_coords: ChunkBlockCoordinate,
        direction_to_check: BlockDirection,
        this_block_id: u16,
        blocks: &Registry<Block>,
    ) -> bool {
        let delta_chunk_coords = direction_to_check.to_chunk_block_coordinates();
        let this_block_scale = chunk.block_scale(chunk_block_coords);

        let Ok(check_coords) = ChunkBlockCoordinate::try_from(chunk_block_coords + delta_chunk_coords) else {
            let scale_ub = self.scale as UnboundCoordinateType;

            let Ok(coords) = BlockCoordinate::try_from(
                self.negative_most_coord
                    + BlockCoordinate::new(
                        chunk_block_coords.x * self.scale,
                        chunk_block_coords.y * self.scale,
                        chunk_block_coords.z * self.scale,
                    )
                    + UnboundBlockCoordinate::new(
                        delta_chunk_coords.x * scale_ub,
                        delta_chunk_coords.y * scale_ub,
                        delta_chunk_coords.z * scale_ub,
                    ),
            ) else {
                return false;
            };

            // 0..2 ranges in for loops below because an LOD is guarenteed to have neighbors
            // that are between 0.5x to 2x the scale of this LOD. For LODs that are 0.5x the size,
            // we need to check a 2x2 square of blocks to ensure this block should never be rendered.
            // If the neighbor is > 0.5x the scale of this LOD, then these redundant checks will not matter.

            let s2 = self.scale / 2;
            match direction_to_check {
                BlockDirection::NegX | BlockDirection::PosX => {
                    for dz in 0..2 {
                        for dy in 0..2 {
                            let (other_block_id, other_block_scale) = self
                                .lod_root
                                .block_id_at_and_scale(coords + BlockCoordinate::new(0, dy * s2, dz * s2), self.lod_root_scale);

                            if check_block_should_render(this_block_id, this_block_scale, other_block_id, other_block_scale, blocks) {
                                return true;
                            }
                        }
                    }
                }
                BlockDirection::NegY | BlockDirection::PosY => {
                    for dz in 0..2 {
                        for dx in 0..2 {
                            let (other_block_id, other_block_scale) = self
                                .lod_root
                                .block_id_at_and_scale(coords + BlockCoordinate::new(dx * s2, 0, dz * s2), self.lod_root_scale);

                            if check_block_should_render(this_block_id, this_block_scale, other_block_id, other_block_scale, blocks) {
                                return true;
                            }
                        }
                    }
                }
                BlockDirection::NegZ | BlockDirection::PosZ => {
                    for dy in 0..2 {
                        for dx in 0..2 {
                            let (other_block_id, other_block_scale) = self
                                .lod_root
                                .block_id_at_and_scale(coords + BlockCoordinate::new(dx * s2, dy * s2, 0), self.lod_root_scale);

                            if check_block_should_render(this_block_id, this_block_scale, other_block_id, other_block_scale, blocks) {
                                return true;
                            }
                        }
                    }
                }
            }

            return false;
        };

        let other_block_id = chunk.block_at(check_coords);
        let other_block_scale = chunk.block_scale(check_coords);
        check_block_should_render(this_block_id, this_block_scale, other_block_id, other_block_scale, blocks)
    }
}

fn check_block_should_render(
    this_block_id: u16,
    this_block_scale: LodBlockSubScale,
    other_block_id: u16,
    other_block_scale: LodBlockSubScale,
    blocks: &Registry<Block>,
) -> bool {
    other_block_id == AIR_BLOCK_ID
        || other_block_scale != this_block_scale
        || (other_block_id != this_block_id && blocks.from_numeric_id(other_block_id).is_see_through())
}

impl ChunkRendererBackend<LodChunk> for LodChunkRenderingChecker<'_> {
    #[inline(always)]
    fn get_texture_index(&self, index: &BlockTextureIndex, neighbors: BlockNeighbors, face: BlockFace) -> Option<TextureIndex> {
        if self.scale > 8 {
            index
                .atlas_index_for_lod(neighbors)
                .map(Some)
                .unwrap_or_else(|| index.atlas_index_from_face(face, neighbors))
        } else {
            index.atlas_index_from_face(face, neighbors)
        }
    }

    fn check_should_render(
        &self,
        chunk: &LodChunk,
        block_here: &Block,
        chunk_block_coords: ChunkBlockCoordinate,
        blocks: &Registry<Block>,
        direction_to_check: BlockDirection,
        _should_connect: &mut bool,
        _rendering_modes: &BlockRenderingModes,
    ) -> bool {
        self.inner_check_should_render(chunk, chunk_block_coords, direction_to_check, block_here.id(), blocks)
    }

    #[inline(always)]
    fn transform_position(&self, chunk: &LodChunk, coords: ChunkBlockCoordinate, _direction: BlockDirection, position: Vec3) -> Vec3 {
        let bs = chunk.block_scale(coords);

        position * Vec3::new(bs.scaling_x, bs.scaling_y, bs.scaling_z) + Vec3::new(bs.x_offset, bs.y_offset, bs.z_offset)
    }
}

#[cfg(test)]
mod test {
    use cosmos_core::{
        block::{BlockProperty, block_direction::ALL_BLOCK_DIRECTIONS},
        structure::chunk::CHUNK_DIMENSIONS,
    };

    use super::*;

    #[test]
    fn test_block_at() {
        const BLOCK_ID: u16 = 1;
        let mut lod_chunk = LodChunk::default();
        let block = Block::new(&[], BLOCK_ID, "a".into(), 0.0, 0.0, 0.0, vec![], vec![]);
        lod_chunk.set_block_at(
            ChunkBlockCoordinate::new(CHUNK_DIMENSIONS - 1, CHUNK_DIMENSIONS - 1, CHUNK_DIMENSIONS - 1).unwrap(),
            &block,
            Default::default(),
        );

        let lod = Lod::Children(Box::new([
            Lod::Single(Box::new(lod_chunk), false),
            Lod::Single(Box::default(), false),
            Lod::Single(Box::default(), false),
            Lod::Single(Box::default(), false),
            Lod::Single(Box::default(), false),
            Lod::Single(Box::default(), false),
            Lod::Single(Box::default(), false),
            Lod::Single(Box::default(), false),
        ]));

        let coords = BlockCoordinate::new(CHUNK_DIMENSIONS - 1, CHUNK_DIMENSIONS - 1, CHUNK_DIMENSIONS - 1);

        assert_eq!(lod.block_id_at_and_scale(coords, 2).0, BLOCK_ID);
        assert_eq!(
            lod.block_id_at_and_scale(BlockCoordinate::new(coords.x * 2, coords.y * 2, coords.z * 2), 4)
                .0,
            BLOCK_ID
        );
        assert_eq!(
            lod.block_id_at_and_scale(BlockCoordinate::new(coords.x * 2 + 1, coords.y * 2 + 1, coords.z * 2 + 1), 4)
                .0,
            BLOCK_ID
        );
        assert_eq!(
            lod.block_id_at_and_scale(BlockCoordinate::new(coords.x * 2 + 2, coords.y * 2 + 2, coords.z * 2 + 2), 4)
                .0,
            0
        );
    }

    #[test]
    fn test_block_at_2() {
        const BLOCK_ID: u16 = 1;
        let mut lod_chunk = LodChunk::default();
        let block = Block::new(&[], BLOCK_ID, "a".into(), 0.0, 0.0, 0.0, vec![], vec![]);
        lod_chunk.set_block_at(ChunkBlockCoordinate::new(0, 0, 0).unwrap(), &block, Default::default());

        let lod = Lod::Children(Box::new([
            Lod::Single(Box::default(), false),
            Lod::Single(Box::default(), false),
            Lod::Single(Box::default(), false),
            Lod::Single(Box::default(), false),
            Lod::Single(Box::default(), false),
            Lod::Single(Box::default(), false),
            Lod::Children(Box::new([
                Lod::Single(Box::default(), false),
                Lod::Single(Box::default(), false),
                Lod::Single(Box::default(), false),
                Lod::Single(Box::default(), false),
                Lod::Single(Box::default(), false),
                Lod::Single(Box::default(), false),
                Lod::Single(Box::default(), false),
                Lod::Single(Box::new(lod_chunk), false),
            ])),
            Lod::Single(Box::default(), false),
        ]));

        let coords = BlockCoordinate::new(3 * CHUNK_DIMENSIONS, 3 * CHUNK_DIMENSIONS, 2 * CHUNK_DIMENSIONS);

        assert_eq!(lod.block_id_at_and_scale(coords, 4).0, BLOCK_ID);
        assert_eq!(lod.block_id_at_and_scale(coords + BlockCoordinate::new(1, 0, 0), 4).0, 0);
    }

    #[test]
    fn test_renderer() {
        const BLOCK_ID: u16 = 1;
        let mut full_lod_chunk = LodChunk::default();
        let block = Block::new(
            &[BlockProperty::Full],
            BLOCK_ID,
            "cosmos:test".into(),
            0.0,
            0.0,
            0.0,
            vec![],
            vec![],
        );

        let mut blocks_registry = Registry::<Block>::new("cosmos:block");
        blocks_registry.register(Block::new(&[], 0, "cosmos:air".into(), 0.0, 0.0, 0.0, vec![], vec![]));
        blocks_registry.register(block.clone());

        for z in 0..CHUNK_DIMENSIONS {
            for y in 0..CHUNK_DIMENSIONS {
                for x in 0..CHUNK_DIMENSIONS {
                    full_lod_chunk.set_block_at(ChunkBlockCoordinate::new(x, y, z).unwrap(), &block, Default::default());
                }
            }
        }

        let mut half_full_lod_chunk = LodChunk::default();
        let block = Block::new(&[], BLOCK_ID, "a".into(), 0.0, 0.0, 0.0, vec![], vec![]);
        for z in 0..CHUNK_DIMENSIONS / 2 {
            for y in 0..CHUNK_DIMENSIONS {
                for x in 0..CHUNK_DIMENSIONS {
                    half_full_lod_chunk.set_block_at(ChunkBlockCoordinate::new(x, y, z * 2).unwrap(), &block, Default::default());
                }
            }
        }

        let lod = Lod::Children(Box::new([
            Lod::Single(Box::new(full_lod_chunk.clone()), false),
            Lod::Single(Box::new(full_lod_chunk.clone()), false),
            Lod::Single(Box::new(full_lod_chunk.clone()), false),
            Lod::Single(Box::new(full_lod_chunk.clone()), false),
            Lod::Single(Box::new(full_lod_chunk.clone()), false),
            Lod::Single(Box::new(half_full_lod_chunk.clone()), false),
            Lod::Children(Box::new([
                Lod::Single(Box::default(), false), // this one should not change the results
                Lod::Single(Box::new(full_lod_chunk.clone()), false),
                Lod::Single(Box::new(full_lod_chunk.clone()), false),
                Lod::Single(Box::new(full_lod_chunk.clone()), false),
                Lod::Single(Box::new(full_lod_chunk.clone()), false),
                Lod::Single(Box::new(full_lod_chunk.clone()), false),
                Lod::Single(Box::new(full_lod_chunk.clone()), false),
                Lod::Single(Box::new(full_lod_chunk.clone()), false),
            ])),
            Lod::Single(Box::new(full_lod_chunk.clone()), false),
        ]));

        let scale = 4;
        let s2 = scale / 2;

        let renderer = LodChunkRenderingChecker {
            lod_root_scale: scale,
            lod_root: &lod,
            scale: 1,
            negative_most_coord: BlockCoordinate::new(s2 * CHUNK_DIMENSIONS, s2 * CHUNK_DIMENSIONS, s2 * CHUNK_DIMENSIONS),
        };

        for z in 0..CHUNK_DIMENSIONS {
            for y in 0..CHUNK_DIMENSIONS {
                for x in 0..CHUNK_DIMENSIONS {
                    for dir in ALL_BLOCK_DIRECTIONS {
                        assert_eq!(
                            renderer.inner_check_should_render(
                                &full_lod_chunk,
                                ChunkBlockCoordinate::new(x, y, z).unwrap(),
                                dir,
                                BLOCK_ID,
                                &blocks_registry
                            ),
                            x == 0 && (z / 2) % 2 == 1 && dir == BlockDirection::NegX
                        );
                    }
                }
            }
        }
    }
}
