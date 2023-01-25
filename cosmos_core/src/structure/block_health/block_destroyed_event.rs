use bevy::prelude::{App, Entity};

use crate::structure::structure_block::StructureBlock;

pub struct BlockDestroyedEvent {
    pub structure_entity: Entity,
    pub block: StructureBlock,
}

pub(crate) fn register(app: &mut App) {
    app.add_event::<BlockDestroyedEvent>();
}
