//! Messages

use bevy::prelude::{App, Entity, Message};

use crate::structure::structure_block::StructureBlock;

/// This event is sent when a block is destroyed
#[derive(Debug, Message)]
pub struct BlockDestroyedMessage {
    /// The structure that had its block destroyed
    pub structure_entity: Entity,
    /// The block that was destroyed
    pub block: StructureBlock,
}

/// This event is sent when a block takes damage
#[derive(Debug, Message)]
pub struct BlockTakeDamageMessage {
    /// The structure that had its block take damage
    pub structure_entity: Entity,
    /// The block that took damage
    pub block: StructureBlock,
    /// The block's new health
    pub new_health: f32,
    /// The entity that caused this damage if there is one
    ///
    /// This is NOT the direct causer (such as a laser or missile), but rather the entity that caused the damage
    /// (such as the ship that fired the laser).
    pub causer: Option<Entity>,
}

pub(super) fn register(app: &mut App) {
    app.add_message::<BlockDestroyedMessage>().add_message::<BlockTakeDamageMessage>();
}
