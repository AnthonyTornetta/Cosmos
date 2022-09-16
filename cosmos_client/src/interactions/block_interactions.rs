use bevy::prelude::{App, Entity};
use cosmos_core::structure::structure::StructureBlock;

pub enum InteractionType {
    Primary,
}

pub struct BlockInteractionEvent {
    structure_block: StructureBlock,
    structure_entity: Entity,
    interaction_type: InteractionType,
}

pub fn register(app: &mut App) {
    app.add_event::<BlockInteractionEvent>();
}
