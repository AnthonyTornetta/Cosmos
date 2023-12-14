//! Handles blocks that have inventories

use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::{Event, EventReader, EventWriter},
        system::{Commands, Query, Res},
    },
    log::warn,
};

use crate::{
    block::{data::BlockData, Block},
    events::block_events::BlockChangedEvent,
    inventory::Inventory,
    registry::Registry,
    structure::{structure_block::StructureBlock, Structure},
};

#[derive(Event)]
/// Sent whenever an entity needs an inventory populated.
///
/// On client, this should be populated by asking the server.
///
/// On server, this should be populated by reading the block data on disk or creating a new inventory.
pub struct PopulateBlockInventoryEvent {
    /// The structure's entity
    pub structure_entity: Entity,
    /// The block
    pub block: StructureBlock,
}

/// Used to process the addition/removal of storage blocks to a structure.
///
/// Sends out the `PopulateBlockInventoryEvent` event when needed.
pub fn on_add_storage(
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    mut evr_block_changed: EventReader<BlockChangedEvent>,
    mut commands: Commands,
    mut ev_writer: EventWriter<PopulateBlockInventoryEvent>,
    mut q_block_data: Query<&mut BlockData>,
) {
    if evr_block_changed.is_empty() {
        return;
    }

    let Some(block) = blocks.from_id("cosmos:storage") else {
        return;
    };

    for ev in evr_block_changed.read() {
        if ev.new_block == ev.old_block {
            continue;
        }

        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        if blocks.from_numeric_id(ev.old_block) == block {
            let coords = ev.block.coords();

            if let Some(data_ent) = structure.block_data(coords) {
                if let Ok(mut block_data) = q_block_data.get_mut(data_ent) {
                    block_data.decrement();
                } else {
                    warn!("Missing BlockData on block data component?");
                }

                if let Some(mut ecmds) = commands.get_entity(data_ent) {
                    ecmds.remove::<Inventory>();
                }
            }
        }

        if blocks.from_numeric_id(ev.new_block) == block {
            ev_writer.send(PopulateBlockInventoryEvent {
                block: ev.block,
                structure_entity: ev.structure_entity,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_add_storage).add_event::<PopulateBlockInventoryEvent>();
}
