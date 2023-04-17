//! Block destroyed event

use bevy::prelude::{App, Entity};

use crate::structure::structure_block::StructureBlock;

/// This event is sent when a block is destroyed
pub struct BlockDestroyedEvent {
    /// The structure that had its block destroyed
    pub structure_entity: Entity,
    /// The block that was destroyed
    pub block: StructureBlock,
}

pub(super) fn register(app: &mut App) {
    app.add_event::<BlockDestroyedEvent>();
}
