use bevy::prelude::{App, Entity, Event, EventReader, EventWriter, Query, Update};

use crate::{
    ecs::mut_events::{MutEvent, MutEventsCommand},
    events::block_events::BlockChangedEvent,
    structure::{coordinates::BlockCoordinate, structure_block::StructureBlock, Structure},
};

use super::ALL_BLOCK_FACES;

#[derive(Debug, Clone, Event, PartialEq, Eq)]
pub struct BlockUpdate {
    structure_entity: Entity,
    block: StructureBlock,
    cancelled: bool,
}

impl BlockUpdate {
    pub fn new(structure_entity: Entity, block: StructureBlock) -> Self {
        Self {
            block,
            structure_entity,
            cancelled: false,
        }
    }

    pub fn structure_entity(&self) -> Entity {
        self.structure_entity
    }

    pub fn block(&self) -> StructureBlock {
        self.block
    }

    pub fn cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

pub fn send_block_updates(
    structure_query: Query<&Structure>,
    mut block_chage_event: EventReader<BlockChangedEvent>,
    mut event_writer: EventWriter<MutEvent<BlockUpdate>>,
) {
    let block_updates = block_chage_event
        .iter()
        .filter_map(|ev| {
            let Ok(structure) = structure_query.get(ev.structure_entity) else {
                return None;
            };

            Some(ALL_BLOCK_FACES.iter().filter_map(|face| {
                let coord = face.direction_coordinates() + ev.block.coords();
                let Ok(coord) = BlockCoordinate::try_from(coord) else {
                    return None;
                };
                if !structure.is_within_blocks(coord) {
                    return None;
                }

                Some(MutEvent::from(BlockUpdate {
                    structure_entity: ev.structure_entity,
                    block: ev.block,
                    cancelled: false,
                }))
            }))
        })
        .flat_map(|x| x);

    event_writer.send_batch(block_updates);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, send_block_updates).add_mut_event::<BlockUpdate>();
}
