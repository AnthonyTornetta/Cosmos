use crate::structure::structure_block::StructureBlock;
use bevy::prelude::App;
use bevy::prelude::Entity;

#[derive(Debug)]
pub struct BlockChangedEvent {
    pub block: StructureBlock,
    pub structure_entity: Entity,
    pub old_block: u16,
    pub new_block: u16,
}

pub fn register(app: &mut App) {
    app.add_event::<BlockChangedEvent>();
}
