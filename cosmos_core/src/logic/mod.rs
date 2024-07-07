//! The game's logic system: for wires, logic gates, etc.

use bevy::{
    app::{App, Update},
    prelude::{
        in_state, Commands, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, Query, Res, States, SystemSet, With, Without,
    },
    reflect::Reflect,
    utils::HashSet,
};
use logic::Logic;
use logic_graph::{LogicGraph, LogicGroup};

use crate::{
    block::{Block, BlockFace, ALL_BLOCK_FACES},
    events::block_events::BlockChangedEvent,
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{coordinates::BlockCoordinate, loading::StructureLoadingSet, structure_block::StructureBlock, Structure},
};

use bevy::prelude::IntoSystemSetConfigs;

pub mod logic;
pub mod logic_graph;

/// The number of bits to shift to set or read the logic on/off value from the [`BlockInfo`] of a block.
/// Equivalently, the bit index of the logic value.
pub const LOGIC_BIT: usize = 7;

#[derive(Debug, Copy, Clone, PartialEq)]
/// Defines the types of logic ports, which read and write logic values.
/// Each block face with a logic connection might be a logic port.
pub enum PortType {
    /// Reads the Boolean value of the logic group adjacent to this face to help compute its internal Boolean value.
    Input,
    /// Writes its internal Boolean value to the logic group adjacent to this face.
    Output,
}

#[derive(Debug, Copy, Clone, PartialEq)]
/// Defines how a block face interacts with adjacent logic blocks.
pub enum LogicConnection {
    /// An input or output port.
    Port(PortType),
    /// Joins adjacent logic groups without interrupting them or having delayed inputs or outputs.
    Wire,
}

#[derive(Debug, Clone)]
/// A block that interacts with the logic system, like wires and gates.
pub struct LogicBlock {
    // Specifies the roles of the 6 block faces, ordered by BlockFace index.
    connections: [Option<LogicConnection>; 6],

    id: u16,
    unlocalized_name: String,
}

impl Identifiable for LogicBlock {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

impl LogicBlock {
    /// Creates a link to a block to define its logic connections.
    pub fn new(block: &Block, connections: [Option<LogicConnection>; 6]) -> Self {
        Self {
            connections,
            id: 0,
            unlocalized_name: block.unlocalized_name().to_owned(),
        }
    }

    /// Convenience method for getting the port type without using the BlockFace index.
    pub fn connection_on(&self, face: BlockFace) -> Option<LogicConnection> {
        self.connections[BlockFace::index(&face)]
    }

    /// Returns an iterator over all block faces with any port.
    pub fn faces(&self) -> impl Iterator<Item = BlockFace> + '_ {
        self.connections
            .iter()
            .enumerate()
            .filter(|(_, maybe_port)| maybe_port.is_some())
            .map(|(idx, _)| BlockFace::from_index(idx))
    }

    /// Returns an iterator over all block faces with the specified port type - for example: input or output.
    pub fn faces_with(&self, connection: Option<LogicConnection>) -> impl Iterator<Item = BlockFace> + '_ {
        self.connections
            .iter()
            .enumerate()
            .filter(move |(_, maybe_connection)| **maybe_connection == connection)
            .map(|(idx, _)| BlockFace::from_index(idx))
    }

    /// Returns an iterator over all of this logic block's faces with input ports.
    pub fn input_faces(&self) -> impl Iterator<Item = BlockFace> + '_ {
        self.faces_with(Some(LogicConnection::Port(PortType::Input)))
    }

    /// Returns an iterator over all of this logic block's faces with output ports.
    pub fn output_faces(&self) -> impl Iterator<Item = BlockFace> + '_ {
        self.faces_with(Some(LogicConnection::Port(PortType::Output)))
    }

    /// Returns an iterator over all of this logic block's faces with wire connections.
    pub fn wire_faces(&self) -> impl Iterator<Item = BlockFace> + '_ {
        self.faces_with(Some(LogicConnection::Wire))
    }

    /// Returns an iterator over all of this logic block's faces with no logic connections.
    pub fn non_logic_faces(&self) -> impl Iterator<Item = BlockFace> + '_ {
        self.faces_with(None)
    }
}

#[derive(Debug, Default, Reflect, Hash, PartialEq, Eq, Clone, Copy)]
/// Represents an input or output connection on the face of a logic block.
pub struct Port {
    /// The coordinates of the logic block.
    pub coords: BlockCoordinate,
    /// Which face of the block this port is on.
    /// Any wires or other ports one step in this direction are connected to this port.
    pub local_face: BlockFace,
}

impl Port {
    /// Convenience constructor for Ports.
    pub fn new(coords: BlockCoordinate, local_face: BlockFace) -> Port {
        Port { coords, local_face }
    }

    /// Convenience method for getting a set of all six ports the block at these coordinates have (one for each face).
    /// HashSet format is needed for some DFS methods.
    pub fn all_for(coords: BlockCoordinate) -> HashSet<Port> {
        HashSet::from_iter(ALL_BLOCK_FACES.map(|face| Port::new(coords, face)))
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

fn logic_block_placed_event_listener(
    mut evr_block_updated: EventReader<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    logic_blocks: Res<Registry<LogicBlock>>,
    mut q_logic: Query<&mut Logic>,
    mut q_structure: Query<&mut Structure>,
    mut evw_logic_output: EventWriter<LogicOutputEvent>,
    mut evw_logic_input: EventWriter<LogicInputEvent>,
) {
    for ev in evr_block_updated.read() {
        // If was logic block, remove from graph.
        if let Some(logic_block) = logic_blocks.from_id(blocks.from_numeric_id(ev.old_block).unlocalized_name()) {
            if let Ok(structure) = q_structure.get_mut(ev.structure_entity) {
                if let Ok(mut logic) = q_logic.get_mut(ev.structure_entity) {
                    logic.remove_logic_block(
                        logic_block,
                        ev.block.coords(),
                        &structure,
                        structure.get_entity().expect("Structure should have entity"),
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
                if let Ok(mut logic) = q_logic.get_mut(ev.structure_entity) {
                    logic.add_logic_block(
                        logic_block,
                        ev.block.coords(),
                        &structure,
                        structure.get_entity().expect("Structure should have entity"),
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

fn add_default_logic(q_needs_wire_graph: Query<Entity, (With<Structure>, Without<Logic>)>, mut commands: Commands) {
    for entity in q_needs_wire_graph.iter() {
        commands.entity(entity).insert(Logic::default());
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
            add_default_logic.in_set(StructureLoadingSet::AddStructureComponents),
            logic_block_placed_event_listener,
        )
            .run_if(in_state(playing_state)),
    )
    .register_type::<LogicGraph>()
    .register_type::<LogicGroup>()
    .add_event::<LogicOutputEvent>()
    .add_event::<LogicInputEvent>();
}
