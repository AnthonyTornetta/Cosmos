//! The game's logic system: for wires, logic gates, etc.

use bevy::{
    app::{App, Update},
    prelude::{
        in_state, Commands, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, Query, Res, States, SystemSet, With, Without,
    },
};
use logic_graph::{LogicBlock, LogicGraph, LogicGroup};

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{loading::StructureLoadingSet, structure_block::StructureBlock, Structure},
};

use bevy::prelude::IntoSystemSetConfigs;

pub mod logic_graph;

/// The number of bits to shift to set or read the logic on/off value from the [`BlockInfo`] of a block.
/// Equivalently, the bit index of the logic value.
pub const LOGIC_BIT: usize = 7;

fn logic_block_placed_event_listener(
    mut evr_block_updated: EventReader<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    logic_blocks: Res<Registry<LogicBlock>>,
    mut q_wire_graph: Query<&mut LogicGraph>,
    mut q_structure: Query<&mut Structure>,
    mut evw_logic_output: EventWriter<LogicOutputEvent>,
    mut evw_logic_input: EventWriter<LogicInputEvent>,
) {
    for ev in evr_block_updated.read() {
        // If was logic block, remove from graph.
        if let Some(logic_block) = logic_blocks.from_id(blocks.from_numeric_id(ev.old_block).unlocalized_name()) {
            if let Ok(structure) = q_structure.get_mut(ev.structure_entity) {
                if let Ok(mut wire_graph) = q_wire_graph.get_mut(ev.structure_entity) {
                    wire_graph.remove_logic_block(
                        logic_block,
                        ev.block.coords(),
                        &structure,
                        &blocks,
                        &logic_blocks,
                        &mut evw_logic_output,
                        &mut evw_logic_input,
                    )
                }
            }
        }

        // If is now logic block, add to graph.
        if let Some(logic_block) = logic_blocks.from_id(blocks.from_numeric_id(ev.new_block).unlocalized_name()) {
            if let Ok(structure) = q_structure.get_mut(ev.structure_entity) {
                if let Ok(mut wire_graph) = q_wire_graph.get_mut(ev.structure_entity) {
                    wire_graph.add_logic_block(
                        logic_block,
                        ev.block.coords(),
                        &structure,
                        &blocks,
                        &logic_blocks,
                        &mut evw_logic_output,
                        &mut evw_logic_input,
                    )
                }
            }
        }
    }
}

#[derive(Event, Debug)]
/// Sent when a block's logic output changes.
/// For example, sent when the block is placed or one frame after its inputs change.
pub struct LogicOutputEvent {
    /// The block coordinates.
    pub block: StructureBlock,
    /// The entity containing the structure and wiregraph this block is in.
    pub entity: Entity,
}

#[derive(Event, Debug)]
/// Sent when a block's logic inputs change.
/// For example, when the block is placed or in the same frame another block with an output [`Port`] in its [`LogicGroup`] changes its output.
pub struct LogicInputEvent {
    /// The block coordinates.
    pub block: StructureBlock,
    /// The entity containing the structure and wiregraph this block is in.
    pub entity: Entity,
}

fn add_default_wire_graph(q_needs_wire_graph: Query<Entity, (With<Structure>, Without<LogicGraph>)>, mut commands: Commands) {
    for entity in q_needs_wire_graph.iter() {
        commands.entity(entity).insert(LogicGraph::default());
    }
}

impl Registry<LogicBlock> {
    /// Gets the logic data for the given block.
    pub fn for_block(&self, block: &Block) -> Option<&LogicBlock> {
        self.from_id(block.unlocalized_name())
    }
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Separates the logic update events into two sets to maintain the timing of logic circuits.
pub enum LogicSystemSet {
    /// All output [`Port`]s. These push their values to their [`LogicGroup`]s first in each frame.
    Producing,
    /// All input [`Port`]s. These pull their values from their [`LogicGroup`]s second in each frame.
    Consuming,
}

pub(super) fn register<T: States>(app: &mut App, playing_state: T) {
    create_registry::<LogicBlock>(app, "cosmos:logic_blocks");

    app.configure_sets(Update, (LogicSystemSet::Producing, LogicSystemSet::Consuming).chain());

    app.add_systems(
        Update,
        (
            add_default_wire_graph.in_set(StructureLoadingSet::AddStructureComponents),
            logic_block_placed_event_listener,
        )
            .run_if(in_state(playing_state)),
    )
    .register_type::<LogicGraph>()
    .register_type::<LogicGroup>()
    .add_event::<LogicOutputEvent>()
    .add_event::<LogicInputEvent>();
}
