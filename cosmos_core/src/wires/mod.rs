use bevy::{
    app::{App, Update},
    prelude::{in_state, Commands, Component, Entity, EventReader, IntoSystemConfigs, OnEnter, Query, Res, ResMut, States, With, Without},
    reflect::Reflect,
    utils::{HashMap, HashSet},
};

use crate::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{coordinates::BlockCoordinate, loading::StructureLoadingSet, Structure},
};

fn wire_place_event_listener(
    mut evr_block_updated: EventReader<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    logic_blocks: Res<Registry<LogicBlock>>,
    mut q_wire_graph: Query<&mut WireGraph>,
    mut q_structure: Query<&mut Structure>,
) {
    for ev in evr_block_updated.read() {
        // If was logic block, remove from graph.
        if let Some(logic_block) = logic_blocks.from_id(blocks.from_numeric_id(ev.old_block).unlocalized_name()) {
            if let Ok(structure) = q_structure.get_mut(ev.structure_entity) {
                if let Ok(mut wire_graph) = q_wire_graph.get_mut(ev.structure_entity) {
                    wire_graph.remove_logic_block(logic_block, ev.block.coords(), &structure, &blocks, &logic_blocks)
                }
            }
        }

        // If is now logic block, add to graph.
        if let Some(logic_block) = logic_blocks.from_id(blocks.from_numeric_id(ev.new_block).unlocalized_name()) {
            if let Ok(structure) = q_structure.get_mut(ev.structure_entity) {
                if let Ok(mut wire_graph) = q_wire_graph.get_mut(ev.structure_entity) {
                    wire_graph.add_logic_block(logic_block, ev.block.coords(), &structure, &blocks, &logic_blocks)
                }
            }
        }
    }
}

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
    pub fn faces<'a>(&'a self) -> impl Iterator<Item = BlockFace> + 'a {
        self.connections
            .iter()
            .enumerate()
            .filter(|(_, maybe_port)| maybe_port.is_some())
            .map(|(idx, _)| BlockFace::from_index(idx))
    }

    /// Returns an iterator over all block faces with the specified port type - for example: input or output.
    pub fn faces_with<'a>(&'a self, connection: Option<LogicConnection>) -> impl Iterator<Item = BlockFace> + 'a {
        self.connections
            .iter()
            .enumerate()
            .filter(move |(_, maybe_connection)| **maybe_connection == connection)
            .map(|(idx, _)| BlockFace::from_index(idx))
    }

    /// Returns an iterator over all of this logic block's faces with input ports.
    pub fn input_faces<'a>(&'a self) -> impl Iterator<Item = BlockFace> + 'a {
        self.faces_with(Some(LogicConnection::Port(PortType::Input)))
    }

    /// Returns an iterator over all of this logic block's faces with output ports.
    pub fn output_faces<'a>(&'a self) -> impl Iterator<Item = BlockFace> + 'a {
        self.faces_with(Some(LogicConnection::Port(PortType::Output)))
    }

    /// Returns an iterator over all of this logic block's faces with wire connections.
    pub fn wire_faces<'a>(&'a self) -> impl Iterator<Item = BlockFace> + 'a {
        self.faces_with(Some(LogicConnection::Wire))
    }

    /// Returns an iterator over all of this logic block's faces with no logic connections.
    pub fn non_logic_faces<'a>(&'a self) -> impl Iterator<Item = BlockFace> + 'a {
        self.faces_with(None)
    }
}

fn register_logic_blocks(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    use LogicConnection as LC;
    if let Some(logic_wire) = blocks.from_id("cosmos:logic_wire") {
        registry.register(LogicBlock::new(logic_wire, [Some(LC::Wire); 6]));
    }
    if let Some(logic_on) = blocks.from_id("cosmos:logic_on") {
        registry.register(LogicBlock::new(logic_on, [Some(LC::Port(PortType::Output)); 6]));
    }
    if let Some(light) = blocks.from_id("cosmos:light") {
        registry.register(LogicBlock::new(light, [Some(LC::Port(PortType::Input)); 6]));
    }
}

impl Registry<LogicBlock> {
    /// Gets the logic data for the given block.
    pub fn for_block(&self, block: &Block) -> Option<&LogicBlock> {
        self.from_id(block.unlocalized_name())
    }
}

#[derive(Debug, Default, Reflect, Hash, PartialEq, Eq, Clone)]
struct LogicGroup {
    on: bool,
    recent_wire_coords: Option<BlockCoordinate>,
}

impl LogicGroup {
    fn new(on: bool, recent_wire_coords: Option<BlockCoordinate>) -> LogicGroup {
        LogicGroup { on, recent_wire_coords }
    }
}

#[derive(Debug, Default, Reflect, Hash, PartialEq, Eq, Clone)]
struct Port {
    coords: BlockCoordinate,
    local_face: BlockFace,
}

impl Port {
    fn new(coords: BlockCoordinate, local_face: BlockFace) -> Port {
        Port { coords, local_face }
    }

    fn all_for(coords: BlockCoordinate) -> HashSet<Port> {
        let mut all = HashSet::new();
        for i in 0..=5 {
            all.insert(Port::new(coords, BlockFace::from_index(i)));
        }
        all
    }
}

#[derive(Debug, Default, Reflect, Component)]
struct WireGraph {
    /// As new logic groups are created, this tracks which ID is the next available.
    next_group_id: usize,
    groups: HashMap<usize, LogicGroup>,
    producers: HashMap<Port, usize>,
    consumers: HashMap<Port, usize>,
}

impl WireGraph {
    fn new_group_id(&mut self) -> usize {
        self.next_group_id += 1;
        self.next_group_id - 1
    }

    fn add_group(&mut self, coords: Option<BlockCoordinate>) -> usize {
        let id = self.new_group_id();
        self.groups.insert(id, LogicGroup::new(false, coords));
        id
    }

    fn add_port(
        &mut self,
        coords: BlockCoordinate,
        face: BlockFace,
        port_type: PortType,
        structure: &Structure,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) {
        let local_face = structure.block_rotation(coords).global_to_local(face);
        // If the neighbor coordinates don't exist, no port is added (and thus no new group).
        if let Ok(neighbor_coords) = coords.step(local_face) {
            let maybe_group = self.find_group(
                neighbor_coords,
                local_face.inverse(),
                structure,
                &mut Port::all_for(coords),
                blocks,
                logic_blocks,
            );
            let group_id = maybe_group.unwrap_or_else(|| self.add_group(None));
            match port_type {
                PortType::Input => &mut self.consumers,
                PortType::Output => &mut self.producers,
            }
            .insert(Port::new(coords, local_face), group_id);
        }
    }

    fn remove_port(
        &mut self,
        coords: BlockCoordinate,
        face: BlockFace,
        port_type: PortType,
        structure: &Structure,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) {
        let local_face = structure.block_rotation(coords).global_to_local(face);

        // If the neighbor coordinates don't exist, no port is removed.
        if let Ok(neighbor_coords) = coords.step(local_face) {
            let port = Port::new(coords, local_face);
            let Some(&group_id) = match port_type {
                PortType::Input => &mut self.consumers,
                PortType::Output => &mut self.producers,
            }
            .get(&port) else {
                return;
            };

            // Check if this port is the last block of its group, and delete the group if so.
            if self
                .find_group(
                    neighbor_coords,
                    local_face.inverse(),
                    structure,
                    &mut Port::all_for(coords),
                    blocks,
                    logic_blocks,
                )
                .is_none()
            {
                self.groups.remove(&group_id);
            }

            // Delete the port.
            match port_type {
                PortType::Input => &mut self.consumers,
                PortType::Output => &mut self.producers,
            }
            .remove(&port);
        }
    }

    fn add_logic_block(
        &mut self,
        logic_block: &LogicBlock,
        coords: BlockCoordinate,
        structure: &Structure,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) {
        // Adding input faces as consumers to their connected group, or a new group if there is no connected group.
        for input_face in logic_block.input_faces() {
            self.add_port(coords, input_face, PortType::Input, structure, blocks, logic_blocks)
        }

        // Adding output faces as consumers to their connected group, or a new group if there is no connected group.
        for output_face in logic_block.output_faces() {
            self.add_port(coords, output_face, PortType::Output, structure, blocks, logic_blocks)
        }

        // Connect wire faces to all existing groups (by creating one new group that includes all adjacent groups).
        if logic_block.wire_faces().count() > 0 {
            let mut group_ids: HashSet<usize> = HashSet::new();

            // Get all adjacent group IDs.
            for wire_face in logic_block.wire_faces() {
                let local_face = structure.block_rotation(coords).global_to_local(wire_face);
                if let Ok(neighbor_coords) = coords.step(local_face) {
                    if let Some(group_id) = self.find_group(
                        neighbor_coords,
                        local_face.inverse(),
                        structure,
                        &mut Port::all_for(coords),
                        blocks,
                        logic_blocks,
                    ) {
                        group_ids.insert(group_id);
                    }
                }
            }

            // Create a group if none exists, add to adjacent group if one exists, or merge all adjacent groups if there are multiple.
            match group_ids.len() {
                0 => {
                    let id = self.new_group_id();
                    self.groups.insert(id, LogicGroup::new(false, Some(coords)));
                }
                1 => drop(self.groups.get_mut(group_ids.iter().next().unwrap()).unwrap().recent_wire_coords = Some(coords)),
                _ => self.merge_adjacent_groups(&group_ids, coords),
            };
        }
    }

    fn remove_logic_block(
        &mut self,
        logic_block: &LogicBlock,
        coords: BlockCoordinate,
        structure: &Structure,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) {
        // Removing input ports from their groups.
        for input_face in logic_block.input_faces() {
            self.remove_port(coords, input_face, PortType::Input, structure, blocks, logic_blocks)
        }

        // Removing output ports from their groups.
        for output_face in logic_block.output_faces() {
            self.remove_port(coords, output_face, PortType::Output, structure, blocks, logic_blocks)
        }

        // For wire faces, 1 connection means just delete the wire. 2+ means delete the wire's group and make a new one for each connection.
        // For now, we just delete the group and start again every time to avoid edge cases.
        if logic_block.wire_faces().count() > 0 {
            let group_id = self
                .find_group_all_faces(logic_block, coords, structure, &mut HashSet::new(), blocks, logic_blocks)
                .expect("Block with 'wire' logic connection should have a logic group.");
            self.groups.remove(&group_id);

            // Setting new group IDs.
            let mut visited = Port::all_for(coords);
            for wire_face in logic_block.wire_faces() {
                let local_face = structure.block_rotation(coords).global_to_local(wire_face);
                let Ok(neighbor_coords) = coords.step(local_face) else {
                    continue;
                };
                self.rename_group(neighbor_coords, local_face.inverse(), structure, &mut visited, blocks, logic_blocks);
            }
        }
    }

    fn merge_adjacent_groups(&mut self, group_ids: &HashSet<usize>, coords: BlockCoordinate) {
        // Rewrite all output and input ports of adjacent groups to use the new ID number.
        let new_group_id = self.new_group_id();
        for group_id in self.producers.values_mut() {
            if group_ids.contains(group_id) {
                *group_id = new_group_id;
            }
        }
        for group_id in self.consumers.values_mut() {
            if group_ids.contains(group_id) {
                *group_id = new_group_id;
            }
        }

        // The new group is on if any of its neighbors were.
        let new_group_on = group_ids.iter().fold(false, |or, group_id| or || self.groups[group_id].on);

        // Remove the old groups.
        for group_id in group_ids {
            self.groups.remove(group_id);
        }

        // Creating the new group. The most recent block added is the current block.
        self.groups.insert(
            new_group_id,
            LogicGroup {
                on: new_group_on,
                recent_wire_coords: Some(coords),
            },
        );
    }

    fn find_group(
        &self,
        coords: BlockCoordinate,
        encountered_local_face: BlockFace,
        structure: &Structure,
        visited: &mut HashSet<Port>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) -> Option<usize> {
        let block = structure.block_at(coords, blocks);
        let Some(logic_block) = logic_blocks.from_id(block.unlocalized_name()) else {
            // Not a logic block.
            return None;
        };

        let encountered_face = structure.block_rotation(coords).local_to_global(encountered_local_face);
        match logic_block.connection_on(encountered_face) {
            Some(LogicConnection::Port(PortType::Input)) => self.consumers.get(&Port::new(coords, encountered_local_face)).copied(),
            Some(LogicConnection::Port(PortType::Output)) => self.producers.get(&Port::new(coords, encountered_local_face)).copied(),
            Some(LogicConnection::Wire) => self
                .groups
                .iter()
                .find_map(|(&id, group)| if group.recent_wire_coords == Some(coords) { Some(id) } else { None })
                .or_else(|| {
                    // This wire block does not tell us what group we're in. Recurse on its neighbors.
                    visited.insert(Port::new(coords, encountered_local_face));
                    for face in logic_block.wire_faces() {
                        let local_face = structure.block_rotation(coords).global_to_local(face);
                        visited.insert(Port::new(coords, local_face));
                        let Ok(neighbor_coords) = coords.step(local_face) else {
                            continue;
                        };
                        if visited.contains(&Port::new(neighbor_coords, local_face.inverse())) {
                            continue;
                        }
                        if let Some(group) =
                            self.find_group(neighbor_coords, local_face.inverse(), structure, visited, blocks, logic_blocks)
                        {
                            return Some(group);
                        }
                    }
                    None
                }),
            None => return None,
        }
    }

    fn find_group_all_faces(
        &self,
        logic_block: &LogicBlock,
        coords: BlockCoordinate,
        structure: &Structure,
        visited: &mut HashSet<Port>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) -> Option<usize> {
        for wire_face in logic_block.wire_faces() {
            let local_face = structure.block_rotation(coords).global_to_local(wire_face);
            if let Some(group_id) = self.find_group(coords, local_face.inverse(), structure, visited, blocks, logic_blocks) {
                return Some(group_id);
            }
        }
        None
    }

    fn rename_group(
        &mut self,
        coords: BlockCoordinate,
        encountered_local_face: BlockFace,
        structure: &Structure,
        visited: &mut HashSet<Port>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) {
        // Renaming on this portion already completed.
        if visited.contains(&Port::new(coords, encountered_local_face)) {
            return;
        }
        let block = structure.block_at(coords, blocks);
        let Some(logic_block) = logic_blocks.from_id(block.unlocalized_name()) else {
            // Not a logic block.
            return;
        };

        // Because it has not yet been visited, the current connected group must still need a new ID.
        let new_group_id = self.new_group_id();

        let encountered_face = structure.block_rotation(coords).local_to_global(encountered_local_face);
        match logic_block.connection_on(encountered_face) {
            Some(LogicConnection::Port(PortType::Input)) => self.consumers.insert(Port::new(coords, encountered_local_face), new_group_id),
            Some(LogicConnection::Port(PortType::Output)) => self.producers.insert(Port::new(coords, encountered_local_face), new_group_id),
            Some(LogicConnection::Wire) => {
                // Recurse to continue marking the ports reachable from this wire.
                visited.insert(Port::new(coords, encountered_local_face));
                for face in logic_block.wire_faces() {
                    let local_face = structure.block_rotation(coords).global_to_local(face);
                    visited.insert(Port::new(coords, local_face));
                    let Ok(neighbor_coords) = coords.step(local_face) else {
                        continue;
                    };
                    if visited.contains(&Port::new(neighbor_coords, local_face.inverse())) {
                        continue;
                    }
                    self.rename_group(neighbor_coords, local_face.inverse(), structure, visited, blocks, logic_blocks)
                }
                None
            }
            None => return,
        };
    }
}

fn add_default_wire_graph(q_needs_wire_graph: Query<Entity, (With<Structure>, Without<WireGraph>)>, mut commands: Commands) {
    for entity in q_needs_wire_graph.iter() {
        commands.entity(entity).insert(WireGraph::default());
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T, playing_state: T) {
    create_registry::<LogicBlock>(app, "cosmos:logic_blocks");

    app.add_systems(OnEnter(post_loading_state), register_logic_blocks)
        .add_systems(
            Update,
            (
                add_default_wire_graph.in_set(StructureLoadingSet::AddStructureComponents),
                wire_place_event_listener,
            )
                .run_if(in_state(playing_state)),
        )
        .register_type::<WireGraph>();
}
