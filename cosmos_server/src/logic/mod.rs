//! The game's logic system: for wires, logic gates, etc.

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};
use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use cosmos_core::{
    block::{
        Block,
        block_direction::{ALL_BLOCK_DIRECTIONS, BlockDirection},
        block_events::BlockEventsSet,
        block_face::BlockFace,
        blocks::COLORS,
        data::BlockData,
    },
    events::block_events::{BlockChangedEvent, BlockDataChangedEvent, BlockDataSystemParams},
    logic::BlockLogicData,
    netty::system_sets::NetworkingSystemsSet,
    prelude::StructureLoadedEvent,
    registry::{Registry, create_registry, identifiable::Identifiable},
    state::GameState,
    structure::{Structure, coordinates::BlockCoordinate, loading::StructureLoadingSet, structure_block::StructureBlock},
};
use logic_driver::{LogicBlockChangedEvent, LogicDriver};
use logic_graph::{LogicGraph, LogicGroup};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub mod logic_driver;
mod logic_graph;
mod specific_blocks;

/// The bits to set or read the logic on/off value from the [`BlockInfo`] of a block.
pub const LOGIC_BIT: u8 = 1 << 7;

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

#[derive(Event, Debug, Clone)]
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

fn logic_block_changed_event_listener(
    mut evr_block_changed: EventReader<BlockChangedEvent>,
    q_block_logic_data: Query<&BlockLogicData>,
    mut evr_structure_loaded: EventReader<StructureLoadedEvent>,
    blocks: Res<Registry<Block>>,
    logic_blocks: Res<Registry<LogicBlock>>,
    logic_wire_colors: Res<Registry<LogicWireColor>>,
    mut q_structure: Query<(&mut Structure, &mut LogicDriver)>,
    q_has_data: Query<(), With<BlockLogicData>>,
    mut q_block_data: Query<&mut BlockData>,
    mut bs_params: BlockDataSystemParams,
    mut evw_queue_logic_output: MessageWriter<QueueLogicOutputEvent>,
    mut evw_queue_logic_input: MessageWriter<QueueLogicInputEvent>,
) {
    let mut structures: HashMap<Entity, Vec<LogicBlockChangedEvent<'_>>> = HashMap::default();

    for sle in evr_structure_loaded.read() {
        let Ok((structure, _)) = q_structure.get(sle.structure_entity) else {
            error!("Not structure w/ logicdriver from structure loaded event!");
            continue;
        };

        let all_blocks = structure.all_blocks_iter(false).flat_map(|block_coord| {
            logic_blocks
                .from_id(structure.block_at(block_coord, &blocks).unlocalized_name())
                .map(|logic_block| LogicBlockChangedEvent {
                    coord: block_coord,
                    old: None,
                    new: Some((logic_block, structure.block_rotation(block_coord))),
                })
        });

        structures.insert(sle.structure_entity, all_blocks.collect::<Vec<_>>());
    }

    for ev in evr_block_changed.read() {
        let mut logic_entry = LogicBlockChangedEvent {
            coord: ev.block.coords(),
            old: None,
            new: None,
        };

        if let Some(logic_block) = logic_blocks.from_id(blocks.from_numeric_id(ev.old_block).unlocalized_name()) {
            logic_entry.old = Some((logic_block, ev.old_block_rotation()));
        }

        if let Some(logic_block) = logic_blocks.from_id(blocks.from_numeric_id(ev.new_block).unlocalized_name()) {
            logic_entry.new = Some((logic_block, ev.new_block_rotation()));
        }

        if logic_entry.old.is_some() || logic_entry.new.is_some() {
            structures.entry(ev.block.structure()).or_default().push(logic_entry);
        }
    }

    for (structure_entity, events) in structures {
        let Ok((mut structure, mut logic)) = q_structure.get_mut(structure_entity) else {
            continue;
        };

        let mut events_by_coords = HashMap::new();

        for ev in &events {
            let &LogicBlockChangedEvent { coord, old, new } = ev;
            if let Some((old_logic_block, old_rotation)) = old {
                logic.remove_logic_block(
                    old_logic_block,
                    old_rotation,
                    coord,
                    &structure,
                    structure_entity,
                    &events_by_coords,
                    &blocks,
                    &logic_blocks,
                    &logic_wire_colors,
                    &mut evw_queue_logic_output,
                    &mut evw_queue_logic_input,
                );
            }

            if let Some((new_logic_block, new_rotation)) = new {
                logic.add_logic_block(
                    new_logic_block,
                    new_rotation,
                    coord,
                    &structure,
                    structure_entity,
                    &events_by_coords,
                    &blocks,
                    &logic_blocks,
                    &logic_wire_colors,
                    &mut evw_queue_logic_output,
                    &mut evw_queue_logic_input,
                );

                if let Some(new_block_data) = structure.query_block_data(coord, &q_block_logic_data).copied() {
                    // Add the logic block's internal data storage to the structure.
                    if new_block_data.0 != 0 {
                        structure.insert_block_data(coord, new_block_data, &mut bs_params, &mut q_block_data, &q_has_data);
                    } else {
                        structure.remove_block_data::<BlockLogicData>(coord, &mut bs_params, &mut q_block_data, &q_has_data);
                    }
                }
            }

            events_by_coords.insert(ev.coord, *ev);
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
    mut evr_block_data_changed: EventReader<BlockDataChangedEvent>,
    logic_blocks: Res<Registry<LogicBlock>>,
    blocks: Res<Registry<Block>>,
    q_structure: Query<&Structure>,
) {
    for ev in evr_queue_logic_output.read().cloned().chain(
        evr_block_data_changed
            .read()
            .filter(|ev| {
                let Ok(s) = q_structure.get(ev.block.structure()) else {
                    return false;
                };

                logic_blocks.contains(s.block_at(ev.block.coords(), &blocks).unlocalized_name())
            })
            .map(|x| QueueLogicOutputEvent(LogicOutputEvent { block: x.block })),
    ) {
        logic_output_event_queue.0.push_back(ev.0);
    }
}

fn send_queued_logic_events(
    mut outputs: ResMut<LogicOutputEventQueue>,
    mut inputs: ResMut<LogicInputEventQueue>,
    mut evw_logic_output: MessageWriter<LogicOutputEvent>,
    mut evw_logic_input: MessageWriter<LogicInputEvent>,
) {
    evw_logic_input.write_batch(inputs.0.drain(..));
    evw_logic_output.write_batch(outputs.0.drain(..));
}

/// Many logic blocks simply push their block logic data to their output ports on
pub fn default_logic_block_output(
    block_name: &str,
    mut evr_logic_output: EventReader<LogicOutputEvent>,
    mut evw_queue_logic_input: MessageWriter<QueueLogicInputEvent>,
    logic_blocks: &Registry<LogicBlock>,
    blocks: &Registry<Block>,
    mut q_structure: Query<(&mut Structure, &mut LogicDriver)>,
    q_logic_data: Query<&BlockLogicData>,
) {
    for ev in evr_logic_output.read() {
        let Ok((structure, mut logic_driver)) = q_structure.get_mut(ev.block.structure()) else {
            continue;
        };
        if structure.block_at(ev.block.coords(), blocks).unlocalized_name() != block_name {
            continue;
        }
        let BlockLogicData(signal) = structure
            .query_block_data(ev.block.coords(), &q_logic_data)
            .copied()
            .unwrap_or_default();
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
    /// If you have any systems that need to run before [`LogicSystemSet::QueueConsumers`] but
    /// still need them to run on the same timer as logic, put it here.
    PreLogicTick,
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
    // Changes to a block's internal logic data caused by consuming new inputs are detected and send producers here.
    // BlockLogicDataUpdate,
    /// All output [`Port`]s. These push their values to their [`LogicGroup`]s second in each logic tick.
    Produce,
}

fn register_logic_groups(mut logic_wire_colors: ResMut<Registry<LogicWireColor>>) {
    // Buses carry all color signals but cannot go into logic gates (as this would require some implicit reduction to a single signal).
    for color in COLORS {
        let colored_wire_name = format!("cosmos:logic_wire_{color}");
        logic_wire_colors.register(LogicWireColor::new(colored_wire_name));
    }
}

impl DefaultPersistentComponent for BlockLogicData {}

pub(super) fn register(app: &mut App) {
    specific_blocks::register(app);

    make_persistent::<BlockLogicData>(app);
    create_registry::<LogicBlock>(app, "cosmos:logic_blocks");
    create_registry::<LogicWireColor>(app, "cosmos:logic_wire_colors");
    app.init_resource::<LogicOutputEventQueue>();
    app.init_resource::<LogicInputEventQueue>();

    app.add_systems(OnEnter(GameState::Loading), register_logic_groups);

    // let run_con = on_timer(Duration::from_millis(1000 / LOGIC_TICKS_PER_SECOND));

    app.configure_sets(
        FixedUpdate,
        (
            LogicSystemSet::PreLogicTick, //.run_if(run_con.clone()),
            LogicSystemSet::EditLogicGraph
                .in_set(BlockEventsSet::ProcessEvents)
                // This may be a bad idea?
                .ambiguous_with(BlockEventsSet::ProcessEvents),
            LogicSystemSet::QueueConsumers,
            LogicSystemSet::QueueProducers,
            (
                LogicSystemSet::SendQueues,
                LogicSystemSet::Consume,
                // LogicSystemSet::BlockLogicDataUpdate,
                LogicSystemSet::Produce,
            )
                .chain(), // .run_if(run_con),
        )
            .in_set(NetworkingSystemsSet::Between)
            .chain(),
    );

    app.add_systems(
        FixedUpdate,
        (
            queue_logic_producers.in_set(LogicSystemSet::QueueProducers),
            queue_logic_consumers.in_set(LogicSystemSet::QueueConsumers),
            send_queued_logic_events.in_set(LogicSystemSet::SendQueues),
            // queue_logic_producers.chain().in_set(LogicSystemSet::BlockLogicDataUpdate),
        )
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        FixedUpdate,
        (
            add_default_logic.in_set(StructureLoadingSet::AddStructureComponents),
            logic_block_changed_event_listener
                .in_set(LogicSystemSet::EditLogicGraph)
                .in_set(StructureLoadingSet::StructureLoaded),
        ),
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

    // app.add_systems(
    // LOADING_SCHEDULE,
    // perform_initial_block_logic_tick
    // .before(LogicSystemSet::EditLogicGraph)
    // .in_set(LoadingSystemSet::DoneLoading),
    // );

    app.configure_sets(
        OnEnter(GameState::PostLoading),
        LogicSystemRegistrySet::RegisterLogicBlocks.ambiguous_with(LogicSystemRegistrySet::RegisterLogicBlocks),
    );
}
