//! The game's logic system: for wires, logic gates, etc.

use bevy::{
    app::{App, Update},
    prelude::{
        in_state, Commands, Component, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, Query, Res, States, SystemSet, With,
        Without,
    },
    reflect::Reflect,
    utils::HashSet,
};
use logic_driver::LogicDriver;
use logic_graph::{LogicGraph, LogicGroup};

use crate::{
    block::{data::BlockData, Block, BlockDirection, BlockFace, ALL_BLOCK_DIRECTIONS},
    events::block_events::{BlockChangedEvent, BlockDataSystemParams},
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{coordinates::BlockCoordinate, loading::StructureLoadingSet, structure_block::StructureBlock, Structure},
};

use bevy::prelude::IntoSystemSetConfigs;

pub mod logic_driver;
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
    /// Right, Left, Top, Bottom, Front, Back.
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
    /// Which direction this port points (accounting for block rotation).
    /// Any wires or other ports one step in this direction are connected to this port.
    pub direction: BlockDirection,
}

impl Port {
    /// Convenience constructor for Ports.
    pub fn new(coords: BlockCoordinate, direction: BlockDirection) -> Port {
        Port { coords, direction }
    }

    /// Convenience method for getting a set of all six ports the block at these coordinates have (one for each face).
    /// HashSet format is needed for some DFS methods.
    pub fn all_for(coords: BlockCoordinate) -> HashSet<Port> {
        HashSet::from_iter(ALL_BLOCK_DIRECTIONS.map(|direction| Port::new(coords, direction)))
    }
}

#[derive(Event, Debug)]
/// Sent when a block's logic output changes.
/// For example, sent when the block is placed or one frame after its inputs change.
pub struct LogicOutputEvent {
    /// The block coordinates.
    pub block: StructureBlock,
    /// The entity containing the structure and logic graph this block is in.
    pub entity: Entity,
}

#[derive(Event, Debug)]
/// Sent when a block's logic inputs change.
/// For example, when the block is placed or in the same frame another block with an output [`Port`] in its [`LogicGroup`] changes its output.
pub struct LogicInputEvent {
    /// The block coordinates.
    pub block: StructureBlock,
    /// The entity containing the structure and logic graph this block is in.
    pub entity: Entity,
}

#[derive(Component, Clone, Copy, Reflect, PartialEq, Eq, Debug, Default)]
/// The logic signal this block is holding. Note: each block might interact with this data slightly differently.
/// Usually, a block with an output port will calculate this value the frame before outputting it and store it here.
pub struct BlockLogicData(pub i32);

impl BlockLogicData {
    /// For Boolean applications. 0 is "off" or "false", anything else is "on" or "true".
    pub fn on(&self) -> bool {
        self.0 != 0
    }
}

fn logic_block_placed_event_listener(
    mut evr_block_changed: EventReader<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    logic_blocks: Res<Registry<LogicBlock>>,
    mut q_logic: Query<&mut LogicDriver>,
    mut q_structure: Query<&mut Structure>,
    mut evw_logic_output: EventWriter<LogicOutputEvent>,
    mut evw_logic_input: EventWriter<LogicInputEvent>,
    q_has_data: Query<(), With<BlockLogicData>>,
    mut q_block_data: Query<&mut BlockData>,
    mut bs_params: BlockDataSystemParams,
) {
    for ev in evr_block_changed.read() {
        // If was logic block, remove from the logic graph.
        if let Some(logic_block) = logic_blocks.from_id(blocks.from_numeric_id(ev.old_block).unlocalized_name()) {
            if let Ok(structure) = q_structure.get_mut(ev.structure_entity) {
                if let Ok(mut logic) = q_logic.get_mut(ev.structure_entity) {
                    println!("Removed block rotation: {:?}", ev.old_block_rotation);
                    logic.remove_logic_block(
                        logic_block,
                        ev.old_block_rotation,
                        ev.block.coords(),
                        &structure,
                        structure.get_entity().expect("Structure should have entity."),
                        &blocks,
                        &logic_blocks,
                        &mut evw_logic_output,
                        &mut evw_logic_input,
                    )
                }
            }
        }

        // If is now logic block, add to the logic graph.
        if let Some(logic_block) = logic_blocks.from_id(blocks.from_numeric_id(ev.new_block).unlocalized_name()) {
            if let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) {
                if let Ok(mut logic) = q_logic.get_mut(ev.structure_entity) {
                    let coords = ev.block.coords();
                    logic.add_logic_block(
                        logic_block,
                        ev.new_block_rotation,
                        coords,
                        &structure,
                        structure.get_entity().expect("Structure should have entity"),
                        &blocks,
                        &logic_blocks,
                        &mut evw_logic_output,
                        &mut evw_logic_input,
                    );
                    // Add the logic block's internal data storage to the structure.
                    structure.insert_block_data(coords, BlockLogicData(0), &mut bs_params, &mut q_block_data, &q_has_data);
                }
            }
        }
    }
}

fn add_default_logic(q_needs_logic_driver: Query<Entity, (With<Structure>, Without<LogicDriver>)>, mut commands: Commands) {
    for entity in q_needs_logic_driver.iter() {
        commands.entity(entity).insert(LogicDriver::default());
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
    /// New [`LogicBlock`]s are added before anyone produces or consumes, so they have a chance to do both in their first frame.
    Place,
    /// All output [`Port`]s. These push their values to their [`LogicGroup`]s first in each frame.
    Produce,
    /// All input [`Port`]s. These pull their values from their [`LogicGroup`]s second in each frame.
    Consume,
}

pub(super) fn register<T: States>(app: &mut App, playing_state: T) {
    create_registry::<LogicBlock>(app, "cosmos:logic_blocks");

    app.configure_sets(
        Update,
        (LogicSystemSet::Place, LogicSystemSet::Produce, LogicSystemSet::Consume).chain(),
    );

    app.add_systems(
        Update,
        (
            add_default_logic.in_set(StructureLoadingSet::AddStructureComponents),
            logic_block_placed_event_listener.in_set(LogicSystemSet::Place),
        )
            .run_if(in_state(playing_state)),
    )
    .register_type::<LogicDriver>()
    .register_type::<LogicGraph>()
    .register_type::<LogicGroup>()
    .add_event::<LogicOutputEvent>()
    .add_event::<LogicInputEvent>();
}
