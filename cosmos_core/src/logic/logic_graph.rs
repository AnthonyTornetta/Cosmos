//! The behavior of the logic system, on a structure by structure basis.

use bevy::{
    prelude::{Component, Entity, EventWriter},
    reflect::Reflect,
    utils::{HashMap, HashSet},
};

use crate::{
    block::{Block, BlockFace, BlockRotation},
    registry::{identifiable::Identifiable, Registry},
    structure::{coordinates::BlockCoordinate, structure_block::StructureBlock, Structure},
};

use super::{LogicInputEvent, LogicOutputEvent};

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

    fn all_for(coords: BlockCoordinate) -> HashSet<Port> {
        let mut all = HashSet::new();
        for i in 0..=5 {
            all.insert(Port::new(coords, BlockFace::from_index(i)));
        }
        all
    }
}

#[derive(Debug, Default, Reflect, PartialEq, Eq, Clone)]
/// A single component of a [`LogicGraph`], connected by wires.
/// If you can reach [`Port`] B from [`Port`] or Wire A, A and B should be in the same LogicGroup.
/// Note: Coordinates are not enough to search through a [`LogicGroup`]. [`BlockFace`] directions matter as well.
pub struct LogicGroup {
    /// The most recently placed wire coordinates, to speed up identifying which group a new block is in.
    /// If this wire is removed, an adjacent wire's coordinates are used. If there are no adjacent wires, it becomes [`None`].
    recent_wire_coords: Option<BlockCoordinate>,
    /// All output [`Port`]s in this group. They update first in each frame, pushing any change in their output values to the consumers.
    pub producers: HashMap<Port, bool>,
    /// All input [`Port`]s in this group. They update second in each frame, using their new input values to recalculate their block's logic formula.
    pub consumers: HashSet<Port>,
}

impl LogicGroup {
    fn new(recent_wire_coords: Option<BlockCoordinate>) -> LogicGroup {
        LogicGroup {
            recent_wire_coords,
            producers: HashMap::new(),
            consumers: HashSet::new(),
        }
    }

    fn new_with_ports(recent_wire_coords: Option<BlockCoordinate>, producers: HashMap<Port, bool>, consumers: HashSet<Port>) -> LogicGroup {
        LogicGroup {
            recent_wire_coords,
            producers,
            consumers,
        }
    }

    /// Returns [`true`] if any of this group's producers are on (producing), [`false`] otherwise.
    pub fn on(&self) -> bool {
        self.producers.values().any(|&x| x)
    }

    /// Changes a producer value and propogates the signal to all consumers if the "on" value of the group has changed.
    pub fn update_producer(&mut self, port: Port, on: bool, evw_logic_input: &mut EventWriter<LogicInputEvent>, entity: Entity) {
        let was_on = self.on();
        self.producers.insert(port, on);

        if self.on() != was_on {
            // Notify the input ports in this port's group.
            for &input_port in self.consumers.iter() {
                evw_logic_input.send(LogicInputEvent {
                    block: StructureBlock::new(input_port.coords),
                    entity,
                });
            }
        }
    }
}

#[derive(Debug, Default, Reflect, Component)]
/// Stores all Boolean logic relationships for a single structure.
pub struct LogicGraph {
    /// As new logic groups are created, this tracks which ID is the next available.
    next_group_id: usize,
    /// Each group, indexed by a unique ID, encompasses one component connected by wires.
    pub groups: HashMap<usize, LogicGroup>,
    /// Tracks which logic group a given output Port (coordinate and face) belong to.
    pub output_port_group_id: HashMap<Port, usize>,
    /// Tracks which logic group a given input Port (coordinate and face) belong to.
    pub input_port_group_id: HashMap<Port, usize>,
}

impl LogicGraph {
    fn new_group_id(&mut self) -> usize {
        self.next_group_id += 1;
        self.next_group_id - 1
    }

    fn new_group(&mut self, coords: Option<BlockCoordinate>) -> usize {
        let id = self.new_group_id();
        self.groups.insert(id, LogicGroup::new(coords));
        id
    }

    fn remove_group(&mut self, id: usize) -> LogicGroup {
        self.groups.remove(&id).expect("Removed logic group should have existed.")
    }

    fn add_port(
        &mut self,
        coords: BlockCoordinate,
        local_face: BlockFace,
        group_id: usize,
        port_type: PortType,
        on: bool,
        entity: Entity,
        evw_logic_output: &mut EventWriter<LogicOutputEvent>,
        evw_logic_input: &mut EventWriter<LogicInputEvent>,
    ) {
        match port_type {
            PortType::Input => &mut self.input_port_group_id,
            PortType::Output => &mut self.output_port_group_id,
        }
        .insert(Port::new(coords, local_face), group_id);

        let logic_group = &mut self
            .groups
            .get_mut(&group_id)
            .expect("Group should have vectors of input and output ports.");
        match port_type {
            PortType::Input => {
                logic_group.consumers.insert(Port::new(coords, local_face));
                evw_logic_input.send(LogicInputEvent {
                    block: StructureBlock::new(coords),
                    entity,
                });
            }
            PortType::Output => {
                logic_group.producers.insert(Port::new(coords, local_face), on);
                evw_logic_output.send(LogicOutputEvent {
                    block: StructureBlock::new(coords),
                    entity,
                });
            }
        };
    }

    fn placed_port(
        &mut self,
        coords: BlockCoordinate,
        global_face: BlockFace,
        port_type: PortType,
        structure: &Structure,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
        evw_logic_output: &mut EventWriter<LogicOutputEvent>,
        evw_logic_input: &mut EventWriter<LogicInputEvent>,
    ) {
        let local_face = structure.block_rotation(coords).global_to_local(global_face);
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
            let group_id = maybe_group.unwrap_or_else(|| self.new_group(None));
            self.add_port(
                coords,
                local_face,
                group_id,
                port_type,
                false,
                structure.get_entity().expect("Structure should have entity."),
                evw_logic_output,
                evw_logic_input,
            );
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
        evw_logic_input: &mut EventWriter<LogicInputEvent>,
    ) {
        let local_face = structure.block_rotation(coords).global_to_local(face);

        // If the neighbor coordinates don't exist, no port is removed.
        if let Ok(neighbor_coords) = coords.step(local_face) {
            let port = Port::new(coords, local_face);
            let Some(&group_id) = match port_type {
                PortType::Input => &mut self.input_port_group_id,
                PortType::Output => &mut self.output_port_group_id,
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
                self.remove_group(group_id);
            } else {
                let logic_group = self
                    .groups
                    .get_mut(&group_id)
                    .expect("Removed logic port's group should have a vector of ports.");
                // Delete it from the set of ports of its group.
                match port_type {
                    PortType::Input => drop(logic_group.consumers.remove(&port)),
                    PortType::Output => drop(logic_group.producers.remove(&port)),
                };

                // Ping all inputs in this group to let them know this output port is gone.
                if port_type == PortType::Output {
                    for &input_port in self.groups.get(&group_id).expect("Port should have logic group.").consumers.iter() {
                        evw_logic_input.send(LogicInputEvent {
                            block: StructureBlock::new(input_port.coords),
                            entity: structure.get_entity().expect("Structure should have entity."),
                        });
                    }
                }
            }

            // Delete the port.
            match port_type {
                PortType::Input => &mut self.input_port_group_id,
                PortType::Output => &mut self.output_port_group_id,
            }
            .remove(&port);
        }
    }

    /// Adds a logic block, along with all of its ports and wire connections, to the graph.
    /// If the added block has wire connections, merges adjacent [`LogicGroup`]s into a single group.
    pub fn add_logic_block(
        &mut self,
        logic_block: &LogicBlock,
        coords: BlockCoordinate,
        structure: &Structure,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
        evw_logic_output: &mut EventWriter<LogicOutputEvent>,
        evw_logic_input: &mut EventWriter<LogicInputEvent>,
    ) {
        // Adding input faces as consumers to their connected group, or a new group if there is no connected group.
        for input_face in logic_block.input_faces() {
            self.placed_port(
                coords,
                input_face,
                PortType::Input,
                structure,
                blocks,
                logic_blocks,
                evw_logic_output,
                evw_logic_input,
            )
        }

        // Adding output faces as consumers to their connected group, or a new group if there is no connected group.
        for output_face in logic_block.output_faces() {
            self.placed_port(
                coords,
                output_face,
                PortType::Output,
                structure,
                blocks,
                logic_blocks,
                evw_logic_output,
                evw_logic_input,
            )
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
                0 => drop(self.new_group(Some(coords))),
                1 => {
                    self.groups.get_mut(group_ids.iter().next().unwrap()).unwrap().recent_wire_coords = Some(coords);
                    drop(())
                }
                _ => self.merge_adjacent_groups(
                    &group_ids,
                    coords,
                    structure.get_entity().expect("Structure should have entity."),
                    evw_logic_input,
                ),
            };
        }
    }

    /// Removes a logic block, along with all of its ports and wire connections, from the graph.
    /// If the removed block has wire connections, might split its [`LogicGroup`] into several disconnected groups.
    pub fn remove_logic_block(
        &mut self,
        logic_block: &LogicBlock,
        coords: BlockCoordinate,
        structure: &Structure,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
        evw_logic_output: &mut EventWriter<LogicOutputEvent>,
        evw_logic_input: &mut EventWriter<LogicInputEvent>,
    ) {
        // Removing input ports from their groups.
        for input_face in logic_block.input_faces() {
            self.remove_port(
                coords,
                input_face,
                PortType::Input,
                structure,
                blocks,
                logic_blocks,
                evw_logic_input,
            )
        }

        // Removing output ports from their groups.
        for output_face in logic_block.output_faces() {
            self.remove_port(
                coords,
                output_face,
                PortType::Output,
                structure,
                blocks,
                logic_blocks,
                evw_logic_input,
            )
        }

        // For wire faces, 1 connection means just delete the wire. 2+ means delete the wire's group and make a new one for each connection.
        // For now, we just delete the group and start again every time to avoid edge cases.
        if logic_block.wire_faces().count() > 0 {
            // Old group ID either comes from being the stored wire coordinate for a group, or searching all your neighbors.
            let old_group_id = self
                .groups
                .iter()
                .find_map(|(&id, group)| if group.recent_wire_coords == Some(coords) { Some(id) } else { None })
                .unwrap_or_else(|| {
                    self.find_group_all_faces(logic_block, coords, structure, &mut Port::all_for(coords), blocks, logic_blocks)
                        .expect("Block with 'wire' logic connection should have a logic group.")
                });
            let was_on = self.groups.get(&old_group_id).expect("Logic group being split should exist.").on();

            // Setting new group IDs.
            let mut visited = Port::all_for(coords);
            for wire_face in logic_block.wire_faces() {
                let local_face = structure.block_rotation(coords).global_to_local(wire_face);
                let Ok(neighbor_coords) = coords.step(local_face) else {
                    continue;
                };
                // For now, takes a new ID for every call, even though some (like air blocks or already visited wires) don't need it.
                let group_id = self.new_group(None);
                let used_new_group = self.rename_group(
                    group_id,
                    neighbor_coords,
                    local_face.inverse(),
                    structure,
                    &mut visited,
                    blocks,
                    logic_blocks,
                    evw_logic_output,
                    evw_logic_input,
                );
                if !used_new_group {
                    self.remove_group(group_id);
                } else {
                    let new_group = self.groups.get(&group_id).expect("New group for created ID should exist");
                    if new_group.on() != was_on {
                        // Update the inputs to every input port in this newly created group, if the value of the group has changed.
                        for &input_port in new_group.consumers.iter() {
                            evw_logic_input.send(LogicInputEvent {
                                block: StructureBlock::new(input_port.coords),
                                entity: structure.get_entity().expect("Structure should have entity."),
                            });
                        }
                    }
                }
            }

            self.remove_group(old_group_id);
        }
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
            Some(LogicConnection::Port(PortType::Input)) => {
                self.input_port_group_id.get(&Port::new(coords, encountered_local_face)).copied()
            }
            Some(LogicConnection::Port(PortType::Output)) => {
                self.output_port_group_id.get(&Port::new(coords, encountered_local_face)).copied()
            }
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
            None => None,
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
            let Ok(neighbor_coords) = coords.step(local_face) else {
                continue;
            };
            if let Some(group_id) = self.find_group(neighbor_coords, local_face.inverse(), structure, visited, blocks, logic_blocks) {
                return Some(group_id);
            }
        }
        None
    }

    fn merge_adjacent_groups(
        &mut self,
        group_ids: &HashSet<usize>,
        coords: BlockCoordinate,
        entity: Entity,
        evw_logic_input: &mut EventWriter<LogicInputEvent>,
    ) {
        // Rewrite all output and input ports of adjacent groups to use the new ID number.
        let new_group_id = self.new_group_id();
        for group_id in self.output_port_group_id.values_mut() {
            if group_ids.contains(group_id) {
                *group_id = new_group_id;
            }
        }

        for group_id in self.input_port_group_id.values_mut() {
            if group_ids.contains(group_id) {
                *group_id = new_group_id;
            }
        }

        // Copying all the producers and consumers from the separate groups.
        let mut producers = HashMap::new();
        let mut consumers = HashSet::new();
        for group_id in group_ids {
            let logic_group = self.groups.get(group_id).expect("Group ID for merging should have a group.");
            producers.extend(logic_group.producers.iter());
            consumers.extend(logic_group.consumers.iter());
        }

        // The new group is on if any of its neighbors were.
        // let new_group_on = group_ids.iter().fold(false, |or, group_id| or || self.groups[group_id].on);

        // Remove the old groups.
        for &group_id in group_ids {
            self.remove_group(group_id);
        }

        // Creating the new group. The most recent block added is the current block.
        self.groups
            .insert(new_group_id, LogicGroup::new_with_ports(Some(coords), producers, consumers));

        // Notify all the input ports in the new group that their group's value may have changed.
        for &input_port in self
            .groups
            .get(&new_group_id)
            .expect("Merged logic group should exist.")
            .consumers
            .iter()
        {
            evw_logic_input.send(LogicInputEvent {
                block: StructureBlock::new(input_port.coords),
                entity,
            });
        }
    }

    /// Explores a logic group using DFS, renaming any ports encountered with a new group ID.
    /// Returns the coordinates of the first wire connection block encountered (if it exists) so it can be added to the new group.
    fn rename_group(
        &mut self,
        new_group_id: usize,
        coords: BlockCoordinate,
        encountered_local_face: BlockFace,
        structure: &Structure,
        visited: &mut HashSet<Port>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
        evw_logic_output: &mut EventWriter<LogicOutputEvent>,
        evw_logic_input: &mut EventWriter<LogicInputEvent>,
    ) -> bool {
        if visited.contains(&Port::new(coords, encountered_local_face)) {
            // Renaming on this portion already completed.
            return false;
        }
        let block = structure.block_at(coords, blocks);
        let Some(logic_block) = logic_blocks.from_id(block.unlocalized_name()) else {
            // Not a logic block.
            return false;
        };

        let encountered_face = structure.block_rotation(coords).local_to_global(encountered_local_face);
        match logic_block.connection_on(encountered_face) {
            Some(LogicConnection::Port(port_type)) => {
                // Deciding whether the port was on or off in the old group.
                let on = match port_type {
                    PortType::Input => false,
                    PortType::Output => {
                        let old_group = self
                            .groups
                            .get(
                                self.output_port_group_id
                                    .get(&Port::new(coords, encountered_local_face))
                                    .expect("Port being renamed should have a previous group ID."),
                            )
                            .expect("Port being renamed should have a previous group.");

                        *old_group
                            .producers
                            .get(&Port::new(coords, encountered_local_face))
                            .expect("Existing output port should be either on or off")
                    }
                };

                // Inserting the port into the port to group ID mapping also removes the old version.
                self.add_port(
                    coords,
                    encountered_local_face,
                    new_group_id,
                    port_type,
                    on,
                    structure.get_entity().expect("Structure should have entity"),
                    evw_logic_output,
                    evw_logic_input,
                );
            }
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
                    self.rename_group(
                        new_group_id,
                        neighbor_coords,
                        local_face.inverse(),
                        structure,
                        visited,
                        blocks,
                        logic_blocks,
                        evw_logic_output,
                        evw_logic_input,
                    );
                }
                // The first wire coords are always set last (so they take effect), the only recursive call is in this arm.
                self.groups
                    .get_mut(&new_group_id)
                    .expect("New logic group for renamed portion should exist.")
                    .recent_wire_coords = Some(coords);
            }
            None => {}
        }
        logic_block.connection_on(encountered_face).is_some()
    }

    /// Returns an array of the Boolean value of the given block's input port groups.
    /// A block face without an input port is assigned false.
    /// Global face means these values are immediately usable for computing a block's logic formula with no further rotations.
    pub fn global_port_inputs(&self, coords: BlockCoordinate, rotation: BlockRotation) -> [bool; 6] {
        rotation.all_local_faces().map(|face| {
            self.input_port_group_id
                .get(&Port::new(coords, face))
                .map(|group_id| {
                    self.groups
                        .get(group_id)
                        .expect("Input port with group ID should have a logic group.")
                        .on()
                })
                .unwrap_or(false)
        })
    }

    /// Convenience method to get the [`LogicGroup`] ID, then the [`LogicGroup`] instance itself.
    pub fn logic_group_of(&self, port: &Port, port_type: PortType) -> Option<&LogicGroup> {
        let group_id = match port_type {
            PortType::Output => &self.output_port_group_id,
            PortType::Input => &self.input_port_group_id,
        }
        .get(port)?;
        Some(
            self.groups
                .get(group_id)
                .expect("Output port with logic group ID should have a logic group."),
        )
    }

    /// Convenience method to get the [`LogicGroup`] ID, then the mutable [`LogicGroup`] instance itself.
    pub fn mut_logic_group_of(&mut self, port: &Port, port_type: PortType) -> Option<&mut LogicGroup> {
        let group_id = match port_type {
            PortType::Output => &mut self.output_port_group_id,
            PortType::Input => &mut self.input_port_group_id,
        }
        .get(port)?;
        Some(
            self.groups
                .get_mut(group_id)
                .expect("Output port with logic group ID should have a logic group."),
        )
    }
}
