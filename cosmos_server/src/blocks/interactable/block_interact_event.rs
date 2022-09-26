use bevy::prelude::{App, Entity};
use cosmos_core::structure::structure::StructureBlock;

pub struct BlockInteractEvent {
    pub block_id: u16,
    pub structure_block: StructureBlock,
    pub structure_entity: Entity,
    pub interactor: Entity,
}

pub fn register(app: &mut App) {
    app.add_event::<BlockInteractEvent>();
}
