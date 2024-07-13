//! Contains all the functionality & information related to structures that are fully loaded at all times.
//!
//! This means that all chunks this structure needs are loaded as long as the structure exists.

use std::ops::{Deref, DerefMut};

use bevy::{
    prelude::{EventWriter, Vec3},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{blocks::AIR_BLOCK_ID, Block, BlockRotation},
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
};

use super::{
    base_structure::BaseStructure,
    block_storage::BlockStorer,
    chunk::Chunk,
    coordinates::{BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate, CoordinateType},
    structure_block::StructureBlock,
    ChunkState, Structure,
};

#[derive(Serialize, Deserialize, Reflect, Debug)]
/// Contains all the functionality & information related to structures that are fully loaded at all times.
///
/// This means that all chunks this structure needs are loaded as long as the structure exists.
pub struct FullStructure {
    base_structure: BaseStructure,
    #[serde(skip)]
    block_bounds: Option<(BlockCoordinate, BlockCoordinate)>,
    #[serde(skip)]
    loaded: bool,
}

impl Deref for FullStructure {
    type Target = BaseStructure;

    fn deref(&self) -> &Self::Target {
        &self.base_structure
    }
}

impl DerefMut for FullStructure {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_structure
    }
}

impl FullStructure {
    /// A full structure has all its chunks loaded at the same time.
    ///
    /// - `dimensions` The x/y/z dimensions of the structure. These do not have to be the same
    pub fn new(dimensions: ChunkCoordinate) -> Self {
        Self {
            base_structure: BaseStructure::new(dimensions),
            block_bounds: None,
            loaded: false,
        }
    }

    /// A static version of [`Self::block_relative_position`]. This is useful if you know
    /// the dimensions of the structure, but don't have access to the structure instance.
    ///
    /// Gets the block's relative position to any structure's transform.
    ///
    /// The width, height, and length should be that structure's width, height, and length.
    pub fn block_relative_position_static(
        coords: BlockCoordinate,
        structure_blocks_width: CoordinateType,
        structure_blocks_height: CoordinateType,
        structure_blocks_length: CoordinateType,
    ) -> Vec3 {
        let xoff = structure_blocks_width as f32 / 2.0;
        let yoff = structure_blocks_height as f32 / 2.0;
        let zoff = structure_blocks_length as f32 / 2.0;

        let xx = coords.x as f32 - xoff;
        let yy = coords.y as f32 - yoff;
        let zz = coords.z as f32 - zoff;

        Vec3::new(xx + 0.5, yy + 0.5, zz + 0.5)
    }

    /// Gets the block's relative position to this structure's transform.
    pub fn block_relative_position(&self, coords: BlockCoordinate) -> Vec3 {
        Self::block_relative_position_static(coords, self.blocks_width(), self.blocks_height(), self.blocks_length())
    }

    /// Sets the block at the given block coordinates.
    ///
    /// * `event_writer` If this is `None`, no event will be generated. A valid usecase for this being `None` is when you are initially loading/generating everything and you don't want a billion events being generated.
    pub fn set_block_at(
        &mut self,
        coords: BlockCoordinate,
        block: &Block,
        block_rotation: BlockRotation,
        blocks: &Registry<Block>,
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        self.base_structure.debug_assert_block_coords_within(coords);

        let old_block = self.block_id_at(coords);
        if blocks.from_numeric_id(old_block) == block && self.block_rotation(coords) == block_rotation {
            return;
        }

        let old_block_rotation = self.block_rotation(coords);

        let chunk_coords = ChunkCoordinate::for_block_coordinate(coords);
        let chunk_block_coords = ChunkBlockCoordinate::for_block_coordinate(coords);
        let mut send_event = false;

        let block_id = block.id();

        if let Some((min, max)) = &self.block_bounds {
            if !(coords.x > min.x && coords.y > min.y && coords.z > min.z && coords.x < max.x && coords.y < max.y && coords.z < max.z) {
                // Recompute these later lazily
                self.block_bounds = None;
            }
        }

        if let Some(chunk) = self.mut_chunk_at(chunk_coords) {
            chunk.set_block_at(chunk_block_coords, block, block_rotation);

            if chunk.is_empty() {
                self.base_structure.unload_chunk(chunk_coords);
            }

            send_event = true;
        } else if block_id != AIR_BLOCK_ID {
            let mut chunk = Chunk::new(chunk_coords);
            chunk.set_block_at(chunk_block_coords, block, block_rotation);
            self.base_structure.chunks.insert(self.base_structure.flatten(chunk_coords), chunk);
            send_event = true;
        }

        if !send_event {
            return;
        }
        let Some(self_entity) = self.base_structure.self_entity else {
            return;
        };
        let Some(event_writer) = event_writer else {
            return;
        };

        event_writer.send(BlockChangedEvent {
            new_block: block.id(),
            old_block,
            structure_entity: self_entity,
            block: StructureBlock::new(coords),
            old_block_rotation,
            new_block_rotation: block_rotation,
        });
    }

    /// Removes the block at the given coordinates
    ///
    /// * `event_writer` If this is None, no event will be generated.
    pub fn remove_block_at(
        &mut self,
        coords: BlockCoordinate,
        blocks: &Registry<Block>,
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        self.set_block_at(
            coords,
            blocks.from_numeric_id(AIR_BLOCK_ID),
            BlockRotation::default(),
            blocks,
            event_writer,
        );
    }

    /// Marks this structure as being completely loaded
    pub fn set_loaded(&mut self) {
        self.loaded = true;
    }

    /// Returns true if the `set_loaded` member function has been called.
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Returns the chunk's state
    pub fn get_chunk_state(&self, coords: ChunkCoordinate) -> ChunkState {
        if !self.is_within_chunks(coords) {
            ChunkState::Invalid
        } else if self.loaded {
            ChunkState::Loaded
        } else {
            ChunkState::Loading
        }
    }

    fn is_within_chunks(&self, coords: ChunkCoordinate) -> bool {
        let (w, h, l) = self.block_dimensions().into();

        coords.x < w && coords.y < h && coords.z < l
    }

    /// Returns if the chunk at these chunk coordinates is fully loaded & empty.
    pub fn has_empty_chunk_at(&self, coords: ChunkCoordinate) -> bool {
        self.get_chunk_state(coords) == ChunkState::Loaded && self.chunk_at(coords).map(|c| c.is_empty()).unwrap_or(true)
    }

    /// Lazily computes & returns the AABB in terms of [`BlockCoordinate`] values for this structure.
    ///
    /// This only accounts for placed blocks, and thus will often be < the maximum dimensions.
    ///
    /// If no blocks are placed on this structure, None is returned.
    pub fn placed_block_bounds(self_as_structure: &mut Structure) -> Option<(BlockCoordinate, BlockCoordinate)> {
        let Structure::Full(fs) = &self_as_structure else {
            panic!("This method can only be used on a full structure!");
        };

        let bb = match fs.block_bounds {
            None => Self::compute_block_bounds(self_as_structure),
            Some(bb) => Some(bb),
        };

        let Structure::Full(fs) = self_as_structure else {
            unreachable!("This is done above, but must be done again to please the borrow checker.");
        };

        fs.block_bounds = bb;

        bb
    }

    fn compute_block_bounds(self_as_structure: &Structure) -> Option<(BlockCoordinate, BlockCoordinate)> {
        let (mut min, mut max) = (
            BlockCoordinate::splat(CoordinateType::MAX),
            BlockCoordinate::splat(CoordinateType::MIN),
        );

        let mut any_blocks = false;
        for b in self_as_structure.all_blocks_iter(false) {
            any_blocks = true;

            let c = b.coords();
            if c.x < min.x {
                min.x = c.x;
            }
            if c.y < min.y {
                min.y = c.y;
            }
            if c.z < min.z {
                min.z = c.z;
            }

            if c.x > max.x {
                max.x = c.x;
            }
            if c.y > max.y {
                max.y = c.y;
            }
            if c.z > max.z {
                max.z = c.z;
            }
        }

        if any_blocks {
            Some((min, max))
        } else {
            None
        }
    }
}
