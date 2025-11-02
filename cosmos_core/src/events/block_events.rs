//! Messages that are related to blocks

use crate::block::block_rotation::BlockRotation;
use crate::events::structure::structure_event::StructureMessage;
use crate::structure::chunk::BlockInfo;
use crate::structure::structure_block::StructureBlock;
use bevy::ecs::event::MessageWriter;
use bevy::ecs::system::Commands;
use bevy::ecs::system::SystemParam;
use bevy::prelude::App;
use bevy::prelude::Entity;
use bevy::prelude::Message;

#[derive(Debug, Message, Clone)]
/// Sent when a block is changed (destroyed or placed)
///
/// This is NOT SENT when a block's data is modified.
/// See [`BlockDataChangedMessage`] for that.
pub struct BlockChangedMessage {
    /// The block that was changed
    ///
    /// The actual block may or may not have been updated yet
    pub block: StructureBlock,
    /// The block that was there before
    pub old_block: u16,
    /// The block that is there now/will be there
    pub new_block: u16,
    /// Old block's rotation
    pub old_block_info: BlockInfo,
    /// New block's rotation
    pub new_block_info: BlockInfo,
}

impl StructureMessage for BlockChangedMessage {
    fn structure_entity(&self) -> Entity {
        self.block.structure()
    }
}

impl BlockChangedMessage {
    /// Computes what the old rotation was from the old [`BlockInfo`]
    pub fn old_block_rotation(&self) -> BlockRotation {
        self.old_block_info.get_rotation()
    }

    /// Computes what the new rotation was from the new [`BlockInfo`]
    pub fn new_block_rotation(&self) -> BlockRotation {
        self.new_block_info.get_rotation()
    }
}

#[derive(Message, Debug, Clone)]
/// Whenever a block's data is changed, this event will be sent.
///
/// Assuming you use `structure.insert_block_data` or `structure.remove_block_data`, this event will automatically be sent.
/// This will be sent on the removal, insertion, and modification of block data. NOTE that if you query_mut block data,
/// the change event will NOT be sent.
pub struct BlockDataChangedMessage {
    /// The block data entity (or None if it was removed)
    pub block_data_entity: Option<Entity>,
    /// The block this is referring to
    pub block: StructureBlock,
}

#[derive(SystemParam)]
/// Bevy SystemParams that the structure needs to properly handle block data
pub struct BlockDataSystemParams<'w, 's> {
    /// Commands
    pub commands: Commands<'w, 's>,
    /// Sent whenever the structure thinks the BlockData is changing
    pub ev_writer: MessageWriter<'w, BlockDataChangedMessage>,
}

pub(super) fn register(app: &mut App) {
    app.add_event::<BlockDataChangedMessage>().add_event::<BlockChangedMessage>();
}
