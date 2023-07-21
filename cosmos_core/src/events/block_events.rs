//! Events that are related to blocks

use crate::block::BlockFace;
use crate::structure::structure_block::StructureBlock;
use bevy::prelude::App;
use bevy::prelude::Entity;
use bevy::prelude::Event;

#[derive(Debug, Event)]
/// Sent when a block is changed (destroyed or placed)
pub struct BlockChangedEvent {
    /// The block that was changed
    ///
    /// The actual block may or may not have been updated yet
    pub block: StructureBlock,
    /// The structure entity
    pub structure_entity: Entity,
    /// The block that was there before
    pub old_block: u16,
    /// The block that is there now/will be there
    pub new_block: u16,
    /// Old block's rotation
    pub old_block_up: BlockFace,
    /// New block's rotation
    pub new_block_up: BlockFace,
}

pub(super) fn register(app: &mut App) {
    app.add_event::<BlockChangedEvent>();
}
