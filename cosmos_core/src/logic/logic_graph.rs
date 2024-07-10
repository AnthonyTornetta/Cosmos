//! The behavior of the logic system, on a structure by structure basis.

use bevy::{
    prelude::{Entity, EventWriter},
    reflect::Reflect,
    utils::{HashMap, HashSet},
};

use crate::{
    block::{Block, BlockFace},
    registry::{identifiable::Identifiable, Registry},
    structure::{coordinates::BlockCoordinate, structure_block::StructureBlock, Structure},
};

use super::{LogicBlock, LogicConnection, LogicInputEvent, LogicOutputEvent, Port, PortType};

#[derive(Debug, Default, Reflect, PartialEq, Eq, Clone)]
/// A single component of a [`LogicGraph`], connected by wires.
/// If you can reach [`Port`] B from [`Port`] or Wire A, A and B should be in the same LogicGroup.
/// Note: Coordinates are not enough to search through a [`LogicGroup`]. [`BlockFace`] directions matter as well.
pub(super) struct LogicGroup {
    /// The most recently placed wire coordinates, to speed up identifying which group a new block is in.
    /// If this wire is removed, an adjacent wire's coordinates are used. If there are no adjacent wires, it becomes [`None`].
    recent_wire_coords: Option<BlockCoordinate>,
    /// All output [`Port`]s in this group. They update first in each frame, pushing any change in their output values to the consumers.
    pub producers: HashMap<Port, i32>,
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

    fn new_with_ports(recent_wire_coords: Option<BlockCoordinate>, producers: HashMap<Port, i32>, consumers: HashSet<Port>) -> LogicGroup {
        LogicGroup {
            recent_wire_coords,
            producers,
            consumers,
        }
    }

    /// The signal on the group, which is the sum of all signals being produced by the group's producers.
    pub fn signal(&self) -> i32 {
        self.producers.values().sum()
    }

    // Any non-zero signal is considered "on".
    pub fn on(&self) -> bool {
        self.signal() != 0
    }

    /// Changes a producer value and propogates the signal to all consumers if the "on" value of the group has changed.
    pub fn update_producer(&mut self, port: Port, signal: i32, evw_logic_input: &mut EventWriter<LogicInputEvent>, entity: Entity) {
        let &old_signal = self.producers.get(&port).expect("Output port to be updated should exist.");
        self.producers.insert(port, signal);

        if self.signal() != old_signal {
            // Notify the input ports in this port's group if the group's total signal has changed.
            for &input_port in self.consumers.iter() {
                evw_logic_input.send(LogicInputEvent {
                    block: StructureBlock::new(input_port.coords),
                    entity,
                });
            }
        }
    }
}

#[derive(Debug, Default, Reflect)]
/// Stores all Boolean logic relationships for a single structure.
/// An entity's [`LogicGraph`] should never be accessed directly, except by the [`super::logic_driver::LogicDriver`].
pub(super) struct LogicGraph {
    /// As new logic groups are created, this tracks which ID is the next available.
    next_group_id: usize,
    /// Each group, indexed by a unique ID, encompasses one component connected by wires.
    groups: HashMap<usize, LogicGroup>,
    /// Tracks which logic group a given output Port (coordinate and face) belong to.
    output_port_group_id: HashMap<Port, usize>,
    /// Tracks which logic group a given input Port (coordinate and face) belong to.
    input_port_group_id: HashMap<Port, usize>,
}

impl LogicGraph {
    fn new_group_id(&mut self) -> usize {
        self.next_group_id += 1;
        self.next_group_id - 1
    }

    pub fn new_group(&mut self, recent_wire_coords: Option<BlockCoordinate>) -> usize {
        let id = self.new_group_id();
        self.groups.insert(id, LogicGroup::new(recent_wire_coords));
        id
    }

    pub fn remove_group(&mut self, group_id: usize) -> LogicGroup {
        self.groups.remove(&group_id).expect("Logic group to be removed should exist.")
    }

    pub fn get_group(&self, group_id: usize) -> &LogicGroup {
        self.groups.get(&group_id).expect("Logic group with requested ID should exist.")
    }

    /// Public convenience method to get the [`LogicGroup`] ID, then the [`LogicGroup`] instance itself.
    /// Returns None if the given [`Port`] and [`PortType`] are not in this logic graph.
    pub fn group_of(&self, port: &Port, port_type: PortType) -> Option<&LogicGroup> {
        let group_id = match port_type {
            PortType::Output => &self.output_port_group_id,
            PortType::Input => &self.input_port_group_id,
        }
        .get(port)?;
        Some(
            self.groups
                .get(group_id)
                .expect("Logic port with a group ID should have a logic group."),
        )
    }

    /// Convenience method to get the [`LogicGroup`] ID, then the mutable [`LogicGroup`] instance itself.
    /// Returns None if the given [`Port`] and [`PortType`] are not in this logic graph.
    fn mut_group_of(&mut self, port: &Port, port_type: PortType) -> Option<&mut LogicGroup> {
        let group_id = match port_type {
            PortType::Output => &mut self.output_port_group_id,
            PortType::Input => &mut self.input_port_group_id,
        }
        .get(port)?;
        Some(
            self.groups
                .get_mut(group_id)
                .expect("Logic port with a group ID should have a logic group."),
        )
    }

    pub fn dfs_for_group(
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

        let encountered_face = structure.block_rotation(coords).global_to_local(encountered_local_face);
        println!("Rotation: {:?}", structure.block_rotation(coords));
        let name = block.unlocalized_name();
        println!("Encountered {name} through global face: {encountered_face} (pointing {encountered_local_face}).");
        match logic_block.connection_on(encountered_face) {
            Some(LogicConnection::Port(PortType::Input)) => {
                println!("Input Port!");
                self.input_port_group_id.get(&Port::new(coords, encountered_local_face)).copied()
            }
            Some(LogicConnection::Port(PortType::Output)) => {
                println!("Output Port!");
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
                            self.dfs_for_group(neighbor_coords, local_face.inverse(), structure, visited, blocks, logic_blocks)
                        {
                            return Some(group);
                        }
                    }
                    None
                }),
            None => None,
        }
    }

    fn group_dfs_all_faces(
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
            if let Some(group_id) = self.dfs_for_group(neighbor_coords, local_face.inverse(), structure, visited, blocks, logic_blocks) {
                return Some(group_id);
            }
        }
        None
    }

    pub fn get_wire_group(
        &self,
        coords: BlockCoordinate,
        logic_block: &LogicBlock,
        structure: &Structure,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) -> usize {
        self.groups
            .iter()
            .find_map(|(&id, group)| if group.recent_wire_coords == Some(coords) { Some(id) } else { None })
            .unwrap_or_else(|| {
                self.group_dfs_all_faces(logic_block, coords, structure, &mut Port::all_for(coords), blocks, logic_blocks)
                    .expect("Logic block with wire connections is not part of any logic group.")
            })
    }

    pub fn set_group_recent_wire(&mut self, group_id: usize, coords: BlockCoordinate) {
        self.groups
            .get_mut(&group_id)
            .expect("Logic group to update recent wire coordinates should exist.")
            .recent_wire_coords = Some(coords);
    }

    pub fn add_port(
        &mut self,
        coords: BlockCoordinate,
        local_face: BlockFace,
        group_id: usize,
        port_type: PortType,
        signal: i32,
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
                logic_group.producers.insert(Port::new(coords, local_face), signal);
                evw_logic_output.send(LogicOutputEvent {
                    block: StructureBlock::new(coords),
                    entity,
                });
            }
        };
    }

    pub fn remove_port(
        &mut self,
        coords: BlockCoordinate,
        global_face: BlockFace,
        port_type: PortType,
        structure: &Structure,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
        evw_logic_input: &mut EventWriter<LogicInputEvent>,
    ) {
        let local_face = structure.block_rotation(coords).global_to_local(global_face);

        // If the neighbor coordinates don't exist, no port is removed.
        let Ok(neighbor_coords) = coords.step(local_face) else {
            return;
        };

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
            .dfs_for_group(
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

    pub fn merge_adjacent_groups(
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
    pub fn rename_group(
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

        let encountered_face = structure.block_rotation(coords).global_to_local(encountered_local_face);
        match logic_block.connection_on(encountered_face) {
            Some(LogicConnection::Port(port_type)) => {
                // Getting the port's output value in the previous group.
                let old_signal = match port_type {
                    PortType::Input => 0,
                    PortType::Output => {
                        let old_group = self
                            .group_of(&Port::new(coords, encountered_local_face), PortType::Output)
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
                    old_signal,
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

    pub fn update_producer(&mut self, port: Port, signal: i32, evw_logic_input: &mut EventWriter<LogicInputEvent>, entity: Entity) {
        let id = self.output_port_group_id.get(&port);
        println!("Output Port ID: {id:?}");
        self.mut_group_of(&port, PortType::Output)
            .expect("Updated logic port should have a logic group ID.")
            .update_producer(port, signal, evw_logic_input, entity);
    }
}
