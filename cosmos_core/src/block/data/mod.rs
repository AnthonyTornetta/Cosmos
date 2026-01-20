//! Handles the backbone for blocks that store their own data, such as containers

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    prelude::BlockCoordinate,
    structure::{coordinates::ChunkBlockCoordinate, structure_block::StructureBlock},
};

pub mod persistence;

#[derive(Component, Clone, Copy, Debug, Serialize, Deserialize, Reflect)]
/// This component indicates an entity that is storing data for a specific block
pub struct BlockData {
    /// Where this block data is on the structure
    pub identifier: BlockDataIdentifier,
    /// The number of important pieces of data. Used to determine when to remove this data
    ///
    /// Use `self::increment` and `self::decrement` to manage this. Make sure to call these when you add/remove data appropriately!
    pub data_count: usize,
}

#[derive(Component, Clone, Copy, Debug, Serialize, Deserialize, Reflect, PartialEq, Eq)]
/// Where this block data is on the structure
pub struct BlockDataIdentifier {
    /// The block this data is for
    pub block: StructureBlock,
    /// The block id that this data is for
    pub block_id: u16,
}

impl BlockData {
    /// Do this whenever you add a piece of data from this block. Removing the data entity will be automatically handled for you if you do this right.
    pub fn increment(&mut self) {
        self.data_count += 1;
    }

    /// Do this whenever you remove a piece of data from this block. Removing the data entity will be automatically handled for you if you do this right.
    pub fn decrement(&mut self) {
        assert_ne!(self.data_count, 0);
        self.data_count -= 1;
    }

    /// Returns true if this [`BlockData`] entity contains no actual data
    pub fn is_empty(&self) -> bool {
        self.data_count == 0
    }

    /// Returns the coordinates this block has on the structure
    pub fn coords(&self) -> BlockCoordinate {
        self.identifier.block.coords()
    }

    /// Returns the structure this block data is a part of
    pub fn structure(&self) -> Entity {
        self.identifier.block.structure()
    }
}

fn name_block_data(query: Query<(Entity, &BlockData), Without<Name>>, mut commands: Commands) {
    for (ent, data) in query.iter() {
        commands.entity(ent).try_insert(Name::new(format!(
            "BlockData for Block @ {}",
            ChunkBlockCoordinate::for_block_coordinate(data.identifier.block.coords())
        )));
    }
}

pub(super) fn register(app: &mut App) {
    persistence::register(app);

    app.add_systems(First, name_block_data).register_type::<BlockData>();
}
