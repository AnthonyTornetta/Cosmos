use crate::structure::StructureBlock;
use bevy::prelude::App;
use bevy::prelude::Entity;

pub struct BlockChangedEvent {
    pub block: StructureBlock,
    pub structure_entity: Entity,
    pub old_block: u16,
    pub new_block: u16,
}

pub fn register(app: &mut App) {
    app.add_event::<BlockChangedEvent>();
}
