//! The game's logic system: for wires, logic gates, etc.

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};
use bevy::{
    prelude::*,
    time::common_conditions::on_timer,
    utils::{HashMap, hashbrown::HashSet},
};
use cosmos_core::{
    block::{
        Block,
        block_direction::{ALL_BLOCK_DIRECTIONS, BlockDirection},
        block_events::BlockEventsSet,
        block_face::BlockFace,
        data::BlockData,
    },
    events::block_events::{BlockChangedEvent, BlockDataChangedEvent, BlockDataSystemParams},
    logic::BlockLogicData,
    netty::system_sets::NetworkingSystemsSet,
    registry::{Registry, create_registry, identifiable::Identifiable},
    state::GameState,
    structure::{Structure, coordinates::BlockCoordinate, loading::StructureLoadingSet, structure_block::StructureBlock},
};
use logic_driver::LogicDriver;
use logic_graph::{LogicGraph, LogicGroup};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, time::Duration};

pub mod logic_driver;
mod logic_graph;
mod specific_blocks;

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
/// Defines the types of logic wires and how they connect to each other.
/// Each block face with a logic connection might be a logic port.
pub enum WireType {
    /// Connects to all colors, for example the logic bus.
    Bus,
    /// Connects to only a single color, identified by the logic wire color registry ID.
    Color(u16),
}

impl WireType {
    /// Returns true if this wire type connects to the given color, false otherwise.
    pub fn connects_to_color(self, id: u16) -> bool {
        match self {
            Self::Bus => true,
            Self::Color(self_color_id) => self_color_id == id,
        }
    }

    /// Returns true if this wire type connects to the given wire type, false otherwise.
    pub fn connects_to_wire_type(self, other: Self) -> bool {
        match self {
            Self::Bus => true,
            Self::Color(self_color_id) => other.connects_to_color(self_color_id),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
/// Defines how a block face interacts with adjacent logic blocks.
pub enum LogicConnection {
    /// An input or output port.
    Port(PortType),
    /// Joins adjacent logic groups without interrupting them or having delayed inputs or outputs.
    Wire(WireType),
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
            id: u16::MAX,
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
        self.connections
            .iter()
            .enumerate()
            .filter(move |(_, maybe_connection)| matches!(**maybe_connection, Some(LogicConnection::Wire(_))))
            .map(|(idx, _)| BlockFace::from_index(idx))
    }

    /// Returns an iterator over all of this logic block's faces with wire connections that connect to the given wire type.
    pub fn wire_faces_connecting_to(&self, wire_type: WireType) -> impl Iterator<Item = BlockFace> + '_ {
        self.connections
            .iter()
            .enumerate()
            .filter(move |(_, maybe_connection)| match **maybe_connection {
                Some(LogicConnection::Wire(encountered_wire_type)) => encountered_wire_type.connects_to_wire_type(wire_type),
                _ => false,
            })
            .map(|(idx, _)| BlockFace::from_index(idx))
    }

    fn wire_face_colors_no_bus(&self) -> impl Iterator<Item = u16> + '_ {
        let color_set: HashSet<u16> = self
            .connections
            .iter()
            .filter_map(|connection| match connection {
                Some(LogicConnection::Wire(WireType::Color(color_id))) => Some(*color_id),
                _ => None,
            })
            .collect();
        color_set.into_iter()
    }

    /// Returns an iterator over every wire color ID any wire face of this block connects to.
    ///
    /// Returns the iterator over all wire color IDs if any of the faces are logic bus.
    pub fn wire_face_colors<'a>(&'a self, logic_wire_colors: &'a Registry<LogicWireColor>) -> Box<dyn Iterator<Item = u16> + 'a> {
        if self.faces_with(Some(LogicConnection::Wire(WireType::Bus))).next().is_some() {
            Box::new(logic_wire_colors.all_ids())
        } else {
            Box::new(self.wire_face_colors_no_bus())
        }
    }

    /// Returns an iterator over all of this logic block's faces with no logic connections.
    pub fn non_logic_faces(&self) -> impl Iterator<Item = BlockFace> + '_ {
        self.faces_with(None)
    }
}

#[derive(Debug, Default, Reflect, Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
/// A type to be registered for each color of logic wire, so their IDs can be used to check connections in the logic graph.
pub struct LogicWireColor {
    id: u16,
    unlocalized_name: String,
}

impl Identifiable for LogicWireColor {
    #[inline]
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    #[inline]
    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

impl LogicWireColor {
    /// Creates a wire color.
    ///
    /// * `unlocalized_name` This should be unique for that block with the following formatting: `mod_id:color_name`. Such as: `cosmos:dark_red`.
    pub fn new(unlocalized_name: String) -> Self {
        Self {
            id: u16::MAX,
            unlocalized_name,
        }
    }
}

impl PartialEq for LogicWireColor {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Event, Debug, Clone)]
/// Sent when a block's logic inputs change.
/// For example, in the same tick another block with an output [`Port`] in its [`LogicGroup`] changes its output.
pub struct LogicInputEvent {
    /// The block coordinates.
    pub block: StructureBlock,
}

#[derive(Event, Debug)]
/// Sent when a block's logic input changes for a reason outside a logic tick, like placing a new logic block.
pub struct QueueLogicInputEvent(pub LogicInputEvent);

impl QueueLogicInputEvent {
    /// Convenience constructor to avoid having to construct the inner type.
    pub fn new(block: StructureBlock) -> Self {
        Self(LogicInputEvent { block })
    }
}

#[derive(Event, Debug, Clone)]
/// Sent when a block's logic output changes.
/// For example, sent when the block is placed or one tick after its inputs change.
pub struct LogicOutputEvent {
    /// The block coordinates.
    pub block: StructureBlock,
}

#[derive(Event, Debug)]
/// Sent when a block's logic output changes for a reason outside a logic tick, like placing a new logic block.
pub struct QueueLogicOutputEvent(pub LogicOutputEvent);

impl QueueLogicOutputEvent {
    /// Convenience constructor to avoid having to construct the inner type.
    pub fn new(block: StructureBlock) -> Self {
        Self(LogicOutputEvent { block })
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Logic blocks should be registered here and can be ambiguous with this set
pub enum LogicSystemRegistrySet {
    /// Logic blocks should be registered here and can be ambiguous with this set
    RegisterLogicBlocks,
}

/// Whenever a block's logic data is modified, this system sends a block output event for that block.
fn listen_for_changed_logic_data(
    blocks: Res<Registry<Block>>,
    logic_blocks: Res<Registry<LogicBlock>>,
    mut evr_block_data_changed: EventReader<BlockDataChangedEvent>,
    mut evw_logic_output: EventWriter<LogicOutputEvent>,
    q_structure: Query<&Structure>,
) {
    evw_logic_output.send_batch(
        evr_block_data_changed
            .read()
            .filter(|ev| {
                let Ok(structure) = q_structure.get(ev.block.structure()) else {
                    return false;
                };
                let id = structure.block_id_at(ev.block.coords());
                logic_blocks.from_id(blocks.from_numeric_id(id).unlocalized_name()).is_some()
            })
            .map(|ev| LogicOutputEvent { block: ev.block }),
    );
}

fn logic_block_changed_event_listener(
    mut evr_block_changed: EventReader<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    logic_blocks: Res<Registry<LogicBlock>>,
    logic_wire_colors: Res<Registry<LogicWireColor>>,
    mut q_logic: Query<&mut LogicDriver>,
    mut q_structure: Query<&mut Structure>,
    q_has_data: Query<(), With<BlockLogicData>>,
    mut q_block_data: Query<&mut BlockData>,
    mut bs_params: BlockDataSystemParams,
    mut evw_queue_logic_output: EventWriter<QueueLogicOutputEvent>,
    mut evw_queue_logic_input: EventWriter<QueueLogicInputEvent>,
) {
    // We group the events by entity so we can track the block changes the previous events made.
    let events = evr_block_changed.read().collect::<Vec<_>>();
    let entities = events.iter().map(|ev| ev.block.structure()).collect::<HashSet<Entity>>();
    for entity in entities {
        let current_entity_events = events.iter().filter(|ev| ev.block.structure() == entity);
        let mut events_by_coords: HashMap<BlockCoordinate, BlockChangedEvent> = HashMap::new();
        for &ev in current_entity_events {
            // If was logic block, remove from the logic graph.
            if let Some(logic_block) = logic_blocks.from_id(blocks.from_numeric_id(ev.old_block).unlocalized_name()) {
                if let Ok(structure) = q_structure.get_mut(ev.block.structure()) {
                    if let Ok(mut logic) = q_logic.get_mut(ev.block.structure()) {
                        logic.remove_logic_block(
                            logic_block,
                            ev.old_block_rotation(),
                            ev.block.coords(),
                            &structure,
                            entity,
                            &events_by_coords,
                            &blocks,
                            &logic_blocks,
                            &logic_wire_colors,
                            &mut evw_queue_logic_output,
                            &mut evw_queue_logic_input,
                        )
                    }
                }
            }

            // If is now logic block, add to the logic graph.
            if let Some(logic_block) = logic_blocks.from_id(blocks.from_numeric_id(ev.new_block).unlocalized_name()) {
                if let Ok(mut structure) = q_structure.get_mut(ev.block.structure()) {
                    if let Ok(mut logic) = q_logic.get_mut(ev.block.structure()) {
                        let coords = ev.block.coords();
                        logic.add_logic_block(
                            logic_block,
                            ev.new_block_rotation(),
                            coords,
                            &structure,
                            entity,
                            &events_by_coords,
                            &blocks,
                            &logic_blocks,
                            &logic_wire_colors,
                            &mut evw_queue_logic_output,
                            &mut evw_queue_logic_input,
                        );
                        // Add the logic block's internal data storage to the structure.
                        structure.insert_block_data(coords, BlockLogicData(0), &mut bs_params, &mut q_block_data, &q_has_data);
                    }
                }
            }

            // Add the event we just processed to the HashMap so we can pretend the structure was updated in the coming iterations DFS.
            events_by_coords.insert(ev.block.coords(), ev.clone());
        }
    }
}

#[derive(Resource, Default)]
struct LogicOutputEventQueue(VecDeque<LogicOutputEvent>);

#[derive(Resource, Default)]
struct LogicInputEventQueue(VecDeque<LogicInputEvent>);

fn queue_logic_consumers(
    mut evr_queue_logic_input: EventReader<QueueLogicInputEvent>,
    mut logic_input_event_queue: ResMut<LogicInputEventQueue>,
) {
    for ev in evr_queue_logic_input.read() {
        logic_input_event_queue.0.push_back(ev.0.clone());
    }
}

fn queue_logic_producers(
    mut evr_queue_logic_output: EventReader<QueueLogicOutputEvent>,
    mut logic_output_event_queue: ResMut<LogicOutputEventQueue>,
) {
    for ev in evr_queue_logic_output.read() {
        logic_output_event_queue.0.push_back(ev.0.clone());
    }
}

fn send_queued_logic_events(
    mut outputs: ResMut<LogicOutputEventQueue>,
    mut inputs: ResMut<LogicInputEventQueue>,
    mut evw_logic_output: EventWriter<LogicOutputEvent>,
    mut evw_logic_input: EventWriter<LogicInputEvent>,
) {
    evw_logic_input.send_batch(inputs.0.drain(..));
    evw_logic_output.send_batch(outputs.0.drain(..));
}

/// Many logic blocks simply push their block logic data to their output ports on
pub fn default_logic_block_output(
    block_name: &str,
    mut evr_logic_output: EventReader<LogicOutputEvent>,
    mut evw_queue_logic_input: EventWriter<QueueLogicInputEvent>,
    logic_blocks: &Registry<LogicBlock>,
    blocks: &Registry<Block>,
    mut q_logic_driver: Query<&mut LogicDriver>,
    mut q_structure: Query<&mut Structure>,
    q_logic_data: Query<&BlockLogicData>,
) {
    for ev in evr_logic_output.read() {
        let Ok(structure) = q_structure.get_mut(ev.block.structure()) else {
            continue;
        };
        if structure.block_at(ev.block.coords(), blocks).unlocalized_name() != block_name {
            continue;
        }
        let Ok(mut logic_driver) = q_logic_driver.get_mut(ev.block.structure()) else {
            continue;
        };
        let Some(&BlockLogicData(signal)) = structure.query_block_data(ev.block.coords(), &q_logic_data) else {
            continue;
        };
        // Could cause performance problems if many of the same logic block are updated in a single frame. Might move this lookup somewhere else.
        let Some(logic_block) = logic_blocks.from_id(block_name) else {
            continue;
        };

        for face in logic_block.output_faces() {
            let port = Port::new(ev.block.coords(), structure.block_rotation(ev.block.coords()).direction_of(face));
            logic_driver.update_producer(port, signal, &mut evw_queue_logic_input, ev.block.structure());
        }
    }
}

fn add_default_logic(q_needs_logic_driver: Query<Entity, (With<Structure>, Without<LogicDriver>)>, mut commands: Commands) {
    for entity in q_needs_logic_driver.iter() {
        commands.entity(entity).insert(LogicDriver::default());
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Separates the logic update events into two sets to maintain the timing of logic circuits.
pub enum LogicSystemSet {
    /// [`LogicBlock`]s are added or removed before anyone produces or consumes, so they have a chance to do both in their first logic tick.
    EditLogicGraph,
    /// If something (like placing a logic block) tries to consume before a logic tick, this adds that event to a queue for later processing.
    QueueConsumers,
    /// If something (like placing a logic block) tries to produce before a logic tick, this adds that event to a queue for later processing.
    QueueProducers,
    /// If something (like placing a logic block) tried to produce or consume on an earlier frame, this sends the event on the next logic tick.
    SendQueues,
    /// All input [`Port`]s. These pull their values from their [`LogicGroup`]s first in each logic tick.
    Consume,
    /// Changes to a block's internal logic data caused by consuming new inputs are detected and send producers here.
    BlockLogicDataUpdate,
    /// All output [`Port`]s. These push their values to their [`LogicGroup`]s second in each logic tick.
    Produce,
}

fn register_logic_groups(mut logic_wire_colors: ResMut<Registry<LogicWireColor>>) {
    let logic_wire_colors_array = [
        "grey",
        "black",
        "dark_grey",
        "white",
        "blue",
        "dark_blue",
        "brown",
        "green",
        "dark_green",
        "orange",
        "dark_orange",
        "pink",
        "dark_pink",
        "purple",
        "dark_purple",
        "red",
        "dark_red",
        "yellow",
        "dark_yellow",
        "mint",
    ];

    // Buses carry all color signals but cannot go into logic gates (as this would require some implicit reduction to a single signal).
    for color in logic_wire_colors_array {
        let colored_wire_name = format!("cosmos:logic_wire_{color}");
        logic_wire_colors.register(LogicWireColor::new(colored_wire_name));
    }
}

impl DefaultPersistentComponent for BlockLogicData {}

pub(super) fn register(app: &mut App) {
    specific_blocks::register(app);

    make_persistent::<LogicDriver>(app);
    make_persistent::<BlockLogicData>(app);

    /// All logic signal production and consumption happens on ticks that occur with this many milliseconds between them.
    pub const LOGIC_TICKS_PER_SECOND: u64 = 20;

    create_registry::<LogicBlock>(app, "cosmos:logic_blocks");
    create_registry::<LogicWireColor>(app, "cosmos:logic_wire_colors");
    app.init_resource::<LogicOutputEventQueue>();
    app.init_resource::<LogicInputEventQueue>();

    app.add_systems(OnEnter(GameState::Loading), register_logic_groups);

    app.configure_sets(
        Update,
        (
            LogicSystemSet::EditLogicGraph
                .in_set(BlockEventsSet::ProcessEvents)
                // This may be a bad idea?
                .ambiguous_with(BlockEventsSet::ProcessEvents),
            LogicSystemSet::QueueConsumers,
            LogicSystemSet::QueueProducers,
            (
                LogicSystemSet::SendQueues,
                LogicSystemSet::Consume,
                LogicSystemSet::BlockLogicDataUpdate,
                LogicSystemSet::Produce,
            )
                .chain()
                .run_if(on_timer(Duration::from_millis(1000 / LOGIC_TICKS_PER_SECOND))),
        )
            .in_set(NetworkingSystemsSet::Between)
            .chain(),
    );

    app.add_systems(
        Update,
        (
            add_default_logic.in_set(StructureLoadingSet::AddStructureComponents),
            logic_block_changed_event_listener.in_set(LogicSystemSet::EditLogicGraph),
            queue_logic_producers.in_set(LogicSystemSet::QueueProducers),
            queue_logic_consumers.in_set(LogicSystemSet::QueueConsumers),
            send_queued_logic_events.in_set(LogicSystemSet::SendQueues),
            listen_for_changed_logic_data.in_set(LogicSystemSet::BlockLogicDataUpdate),
        )
            .run_if(in_state(GameState::Playing)),
    )
    .register_type::<LogicDriver>()
    .register_type::<LogicGraph>()
    .register_type::<LogicGroup>()
    .add_event::<LogicInputEvent>()
    .add_event::<LogicOutputEvent>()
    .add_event::<QueueLogicInputEvent>()
    .add_event::<QueueLogicOutputEvent>();

    // TODO: Move this all to server, then add them to LogicSystemRegistrySet::RegisterLogicBlocks.
    app.allow_ambiguous_resource::<Registry<LogicBlock>>();

    app.configure_sets(
        OnEnter(GameState::PostLoading),
        LogicSystemRegistrySet::RegisterLogicBlocks.ambiguous_with(LogicSystemRegistrySet::RegisterLogicBlocks),
    );
}
