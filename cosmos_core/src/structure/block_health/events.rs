//! Events

use bevy::prelude::{App, Entity, Event};

use crate::structure::structure_block::StructureBlock;

/// This event is sent when a block is destroyed
#[derive(Debug, Event)]
pub struct BlockDestroyedEvent {
    /// The structure that had its block destroyed
    pub structure_entity: Entity,
    /// The block that was destroyed
    pub block: StructureBlock,
}

/// This event is sent when a block is destroyed
#[derive(Debug, Event)]
pub struct BlockTakeDamageEvent {
    /// The structure that had its block destroyed
    pub structure_entity: Entity,
    /// The block that took damage
    pub block: StructureBlock,
    /// The block's new health
    pub new_health: f32,
}

pub(super) fn register(app: &mut App) {
    app.add_event::<BlockDestroyedEvent>().add_event::<BlockTakeDamageEvent>();
}
