//! Responsible for the default generation of biospheres.

use crate::{block::Block, registry::Registry, structure::coordinates::CoordinateType};
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

/// Stores which blocks make up each biosphere, and how far below the top solid block each block generates.
/// Blocks in ascending order ("stone" = 5 first, "grass" = 0 last).
#[derive(Resource, Clone, Default, Debug, Serialize, Deserialize)]
pub struct BlockLayers {
    ranges: Vec<(Block, BlockLayer)>,
}

impl BlockLayers {
    /// Returns an iterator over all the block ranges in the order they were added
    pub fn ranges(&self) -> std::slice::Iter<(Block, BlockLayer)> {
        self.ranges.iter()
    }

    /// Gets the block that should appear at this depth level. 0 would be the top block.
    pub fn block_for_depth(&self, depth: u64) -> &Block {
        let mut itr = self.ranges();
        let mut cur_block = &itr.next().expect("This block range has no blocks!").0;

        for (next_block, next_layer) in itr {
            if next_layer.middle_depth > depth {
                break;
            } else {
                cur_block = next_block;
            }
        }

        cur_block
    }
}

/// Stores the blocks and all the noise information for creating the top of their layer.
/// For example, the "stone" BlockLevel has the noise paramters that create the boundry between dirt and stone.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockLayer {
    /// How far away from this elevation should this generate
    ///
    /// For the first block, this should almost always be 0.
    ///
    /// For example:
    /// - `Grass` 0
    /// - `Dirt` 1
    /// - `Stone` 4
    ///
    /// Would create 1 top layer of grass starting at the proper elevation, 3 layers of dirt below that, and however many layers of stone till the bottom
    pub middle_depth: CoordinateType,
    /// How much each change in coordinate will effect the change of the block
    ///
    /// Lower number = less change per block.
    pub delta: f64,
    /// Maximum/minimum height of this layer.
    pub amplitude: f64,
    /// # of iterations for this layer. More = more computationally expensive but better looking terrain.
    ///
    /// I would recommend putting iterations to something like 9 for top-level terrain, and keeping it 1 for everything else.
    pub iterations: usize,
}

impl BlockLayer {
    /// This layer doesn't use a noise function to generate its span, and is thus fixed at a certain depth.
    pub fn fixed_layer(middle_depth: CoordinateType) -> Self {
        Self {
            middle_depth,
            delta: 0.0,
            amplitude: 0.0,
            iterations: 0,
        }
    }
}

#[derive(Debug)]
/// Errors generated when initally setting up the block ranges
pub enum BlockRangeError {
    /// This means the block id provided was not found in the block registry
    MissingBlock(BlockLayers),
}

impl BlockLayers {
    /// Creates a new block range, for each planet type to specify its blocks.
    pub fn new() -> Self {
        Self::default()
    }

    /// Use this to construct the various ranges of the blocks.
    ///
    /// The order you add the ranges in DOES matter.
    ///
    /// middle_depth represents how many blocks from the previous layer this block will appear.
    /// For example, If grass was 0, dirt was 1, and stone was 4, it would generate as:
    /// - Grass
    /// - Dirt
    /// - Dirt
    /// - Dirt
    /// - Dirt
    /// - Stone
    /// - Stone
    /// - Stone
    /// - ... stone down to the bottom
    pub fn add_fixed_layer(
        mut self,
        block_id: &str,
        block_registry: &Registry<Block>,
        middle_depth: CoordinateType,
    ) -> Result<Self, BlockRangeError> {
        let Some(block) = block_registry.from_id(block_id) else {
            return Err(BlockRangeError::MissingBlock(self));
        };
        let layer = BlockLayer::fixed_layer(middle_depth);
        self.ranges.push((block.clone(), layer));
        Ok(self)
    }
}
