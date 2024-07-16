//! Events sent to adjacent blocks on block changes

use bevy::prelude::{App, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, Query, Update};

use crate::{
    ecs::mut_events::{MutEvent, MutEventsCommand},
    events::block_events::BlockChangedEvent,
    structure::{coordinates::BlockCoordinate, structure_block::StructureBlock, Structure},
};

use super::{block_events::BlockEventsSet, ALL_BLOCK_FACES};

#[derive(Debug, Clone, Copy, Event, PartialEq, Eq)]
/// This event is sent whenever an adjacent block is changed
pub struct BlockUpdate {
    structure_entity: Entity,
    block: StructureBlock,
    cancelled: bool,
}

impl BlockUpdate {
    /// Creates a new block update
    pub fn new(structure_entity: Entity, block: StructureBlock) -> Self {
        Self {
            block,
            structure_entity,
            cancelled: false,
        }
    }

    /// The structure that was updated
    pub fn structure_entity(&self) -> Entity {
        self.structure_entity
    }

    /// The block that was changed
    pub fn block(&self) -> StructureBlock {
        self.block
    }

    /// If the event has been cancelled
    pub fn cancelled(&self) -> bool {
        self.cancelled
    }

    /// Cancels the event (will do nothing)
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Sets if the event has been cancelled or not
    pub fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

/// Sends block updates when blocks are changed
pub fn send_block_updates(
    structure_query: Query<&Structure>,
    mut block_chage_event: EventReader<BlockChangedEvent>,
    mut event_writer: EventWriter<MutEvent<BlockUpdate>>,
) {
    let block_updates = block_chage_event
        .read()
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
                    block: StructureBlock(coord),
                    cancelled: false,
                }))
            }))
        })
        .flatten();

    event_writer.send_batch(block_updates);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, send_block_updates.in_set(BlockEventsSet::SendBlockUpdateEvents))
        .add_mut_event::<BlockUpdate>();
}
