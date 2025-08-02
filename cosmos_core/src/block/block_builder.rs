//! Used to more easily create blocks

use crate::block::{Block, BlockProperty};

use super::ConnectionGroup;

/// Used to more easily create blocks
pub struct BlockBuilder {
    properties: Vec<BlockProperty>,
    unlocalized_name: String,
    density: f32,
    hardness: f32,
    mining_resistance: f32,
    connect_to_groups: Vec<ConnectionGroup>,
    connection_groups: Vec<ConnectionGroup>,
    category: Option<String>,
    interactable: bool,
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
            connect_to_groups: vec![],
            connection_groups: vec![],
            category: None,
            interactable: false,
        }
    }

    /// Sets the category of the item that will represent this block.
    ///
    /// See [`crate::item::item_category::ItemCategory`]
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Enables the flag that this block is interactable
    pub fn with_interactable(mut self) -> Self {
        self.interactable = true;
        self
    }

    /// Adds a [`super::ConnectionGroup`] that this block will connect to. You can call this multiple times to connect
    /// to multiple groups of blocks.
    ///
    /// Note you still need to provide the proper art files & potentially collider data for it to look and act linked.
    pub fn connect_to_group(mut self, connection_group: impl Into<ConnectionGroup>) -> Self {
        self.connect_to_groups.push(connection_group.into());

        self
    }

    /// Adds a [`super::ConnectionGroup`] that his block is a part of.
    ///
    /// You can be part of multiple connection groups.
    pub fn add_connection_group(mut self, connection_group: impl Into<ConnectionGroup>) -> Self {
        self.connection_groups.push(connection_group.into());

        self
    }

    /// Adds a property to this block
    pub fn add_property(mut self, prop: BlockProperty) -> Self {
        self.properties.push(prop);

        self
    }

    /// Sets the density of the block
    pub fn set_density(mut self, density: f32) -> Self {
        self.density = density;

        self
    }

    /// Creates that block
    pub fn create(self) -> Block {
        Block::new(
            &self.properties,
            u16::MAX,
            self.unlocalized_name.clone(),
            self.density,
            self.hardness,
            self.mining_resistance,
            self.connect_to_groups,
            self.connection_groups,
            self.category,
            self.interactable,
        )
    }
}
