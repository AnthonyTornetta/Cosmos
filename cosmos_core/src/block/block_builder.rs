//! Used to more easily create blocks

use crate::block::{Block, BlockProperty};

/// Used to more easily create blocks
pub struct BlockBuilder {
    properties: Vec<BlockProperty>,
    unlocalized_name: String,
    density: f32,
    hardness: f32,
    mining_resistance: f32,
}

impl BlockBuilder {
    /// Starts the building process for a block
    ///
    /// * `unlocalized_name` This should be unique for that block with the following formatting: `mod_id:block_identifier`. Such as: `cosmos:laser_cannon`
    pub fn new(unlocalized_name: impl Into<String>, density: f32, hardness: f32, mining_resistance: f32) -> Self {
        Self {
            properties: Vec::new(),
            unlocalized_name: unlocalized_name.into(),
            density,
            hardness,
            mining_resistance,
        }
    }

    /// Adds a property to this block
    pub fn add_property(&mut self, prop: BlockProperty) -> &mut Self {
        self.properties.push(prop);

        self
    }

    /// Sets the density of the block
    pub fn set_density(&mut self, density: f32) -> &mut Self {
        self.density = density;

        self
    }

    /// Creates that block
    pub fn create(&self) -> Block {
        Block::new(
            &self.properties,
            u16::MAX,
            self.unlocalized_name.clone(),
            self.density,
            self.hardness,
            self.mining_resistance,
        )
    }
}
