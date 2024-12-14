//! The behavior of the logic system, on a structure by structure basis.

use bevy::{
    prelude::{Entity, EventWriter},
    reflect::Reflect,
    utils::{HashMap, HashSet},
};

use crate::{
    block::{block_direction::BlockDirection, Block},
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{coordinates::BlockCoordinate, structure_block::StructureBlock, Structure},
};

use super::{LogicBlock, LogicConnection, Port, PortType, QueueLogicInputEvent, QueueLogicOutputEvent, WireType};

#[derive(Debug, Default, Reflect, PartialEq, Eq, Clone)]
/// A single component of a [`LogicGraph`], connected by wires.
/// If you can reach [`Port`] B from [`Port`] or Wire A, A and B should be in the same LogicGroup.
/// Note: Coordinates are not enough to search through a [`LogicGroup`]. [`BlockFace`] directions matter as well.
pub(super) struct LogicGroup {
    /// Each logic group should have wires of a single color (buses have several groups).
    /// Only None if there are no wires (for example, if the group has only a single block with only input/output ports).
    pub wire_color_id: Option<u16>,
    /// The most recently placed wire coordinates, to speed up identifying which group a new block is in.
    /// If this wire is removed, an adjacent wire's coordinates are used. If there are no adjacent wires, it becomes [`None`].
    recent_wire_coords: Option<BlockCoordinate>,
    /// All output [`Port`]s in this group. They update first in each frame, pushing any change in their output values to the consumers.
    /// The integer value is the output port's current logic value.
    pub producers: HashMap<Port, i32>,
    /// All input [`Port`]s in this group. They update second in each frame, using their new input values to recalculate their block's logic formula.
    pub consumers: HashSet<Port>,
}

impl LogicGroup {
    fn new(wire_color_id: Option<u16>, recent_wire_coords: Option<BlockCoordinate>) -> LogicGroup {
        LogicGroup {
            wire_color_id,
            recent_wire_coords,
            producers: HashMap::new(),
            consumers: HashSet::new(),
        }
    }

    fn new_with_ports(
        wire_color_id: Option<u16>,
        recent_wire_coords: Option<BlockCoordinate>,
        producers: HashMap<Port, i32>,
        consumers: HashSet<Port>,
    ) -> LogicGroup {
        LogicGroup {
            wire_color_id,
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
    pub fn update_producer(
        &mut self,
        port: Port,
        signal: i32,
        evw_queue_logic_input: &mut EventWriter<QueueLogicInputEvent>,
        entity: Entity,
    ) {
        let &old_signal = self.producers.get(&port).expect("Output port to be updated should exist.");
        self.producers.insert(port, signal);

        if self.signal() != old_signal {
            // Notify the input ports in this port's group if the group's total signal has changed.
            evw_queue_logic_input.send_batch(
                self.consumers
                    .iter()
                    .map(|input_port| QueueLogicInputEvent::new(StructureBlock::new(input_port.coords, entity))),
            );
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

    pub fn new_group(&mut self, wire_color_id: Option<u16>, recent_wire_coords: Option<BlockCoordinate>) -> usize {
        let id = self.new_group_id();
        self.groups.insert(id, LogicGroup::new(wire_color_id, recent_wire_coords));
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
        Some(self.groups.get(group_id).unwrap_or_else(|| {
            panic!(
                "Logic {:?} port at {:?} with a group ID {:?} should have a logic group.",
                port_type, port, group_id
            )
        }))
    }

    /// Convenience method to get the [`LogicGroup`] ID, then the mutable [`LogicGroup`] instance itself.
    /// Returns None if the given [`Port`] and [`PortType`] are not in this logic graph.
    fn mut_group_of(&mut self, port: &Port, port_type: PortType) -> Option<&mut LogicGroup> {
        let group_id = match port_type {
            PortType::Output => &mut self.output_port_group_id,
            PortType::Input => &mut self.input_port_group_id,
        }
        .get(port)?;
        Some(self.groups.get_mut(group_id).unwrap_or_else(|| {
            panic!(
                "Logic {:?} port at {:?} with a group ID {:?} should have a mutable logic group.",
                port_type, port, group_id
            )
        }))
    }

    /// `LogicGraph`'s `dfs_for_group` method needs to know which blocks are at each coordinate to properly search the graph.
    ///
    /// However, several `BlockChangedEvent`s can occur on the same tick.
    /// We use `events_by_coords` to track all the events the logic graph has alredy processed this tick, and pretend the blocks have already been changed in the structure.
    fn block_at<'a>(
        &self,
        coords: BlockCoordinate,
        structure: &'a Structure,
        events_by_coords: &HashMap<BlockCoordinate, BlockChangedEvent>,
        blocks: &'a Registry<Block>,
    ) -> &'a Block {
        if let Some(ev) = events_by_coords.get(&coords) {
            return blocks.from_numeric_id(ev.new_block);
        }
        structure.block_at(coords, blocks)
    }

    pub fn dfs_for_group(
        &self,
        coords: BlockCoordinate,
        encountered_from_direction: BlockDirection,
        mut required_color_id: Option<u16>,
        from_bus: bool,
        structure: &Structure,
        events_by_coords: &HashMap<BlockCoordinate, BlockChangedEvent>,
        visited: &mut HashSet<Port>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) -> Option<usize> {
        let block = self.block_at(coords, structure, events_by_coords, blocks);
        let Some(logic_block) = logic_blocks.from_id(block.unlocalized_name()) else {
            // Not a logic block.
            return None;
        };

        let encountered_face = structure.block_rotation(coords).block_face_pointing(encountered_from_direction);
        match logic_block.connection_on(encountered_face) {
            Some(LogicConnection::Port(PortType::Input)) => {
                if !from_bus {
                    self.input_port_group_id
                        .get(&Port::new(coords, encountered_from_direction))
                        .copied()
                } else {
                    None
                }
            }
            Some(LogicConnection::Port(PortType::Output)) => {
                if !from_bus {
                    self.output_port_group_id
                        .get(&Port::new(coords, encountered_from_direction))
                        .copied()
                } else {
                    None
                }
            }
            Some(LogicConnection::Wire(wire_type)) => {
                // Logic buses should not interact with logic that doesn't have a wire color.
                if required_color_id.is_none() && wire_type == WireType::Bus {
                    return None;
                }
                let wire_color_id = *required_color_id.get_or_insert(match wire_type {
                    WireType::Bus => u16::MAX, // If statement above means this value should never be used.
                    WireType::Color(id) => id,
                });
                if wire_type.connects_to_color(wire_color_id) {
                    self.groups
                        .iter()
                        .find_map(|(&id, group)| {
                            if group.wire_color_id == Some(wire_color_id) && group.recent_wire_coords == Some(coords) {
                                Some(id)
                            } else {
                                None
                            }
                        })
                        .or_else(|| {
                            // This wire block does not tell us what group we're in. Recurse on its neighbors.
                            visited.insert(Port::new(coords, encountered_from_direction));
                            for face in logic_block.wire_faces_connecting_to(wire_type) {
                                let direction = structure.block_rotation(coords).direction_of(face);
                                visited.insert(Port::new(coords, direction));
                                let Ok(neighbor_coords) = coords.step(direction) else {
                                    continue;
                                };
                                if visited.contains(&Port::new(neighbor_coords, direction.inverse())) {
                                    continue;
                                }
                                if let Some(group) = self.dfs_for_group(
                                    neighbor_coords,
                                    direction.inverse(),
                                    Some(wire_color_id),
                                    wire_type == WireType::Bus,
                                    structure,
                                    events_by_coords,
                                    visited,
                                    blocks,
                                    logic_blocks,
                                ) {
                                    return Some(group);
                                }
                            }
                            None
                        })
                } else {
                    None
                }
            }
            None => None,
        }
    }

    fn group_dfs_all_faces(
        &self,
        logic_block: &LogicBlock,
        wire_color_id: u16,
        coords: BlockCoordinate,
        structure: &Structure,
        events_by_coords: &HashMap<BlockCoordinate, BlockChangedEvent>,
        visited: &mut HashSet<Port>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) -> Option<usize> {
        for wire_face in logic_block.wire_faces_connecting_to(WireType::Color(wire_color_id)) {
            let direction = structure.block_rotation(coords).direction_of(wire_face);
            let Ok(neighbor_coords) = coords.step(direction) else {
                continue;
            };
            if let Some(group_id) = self.dfs_for_group(
                neighbor_coords,
                direction.inverse(),
                Some(wire_color_id),
                logic_block.connection_on(wire_face) == Some(LogicConnection::Wire(WireType::Bus)),
                structure,
                events_by_coords,
                visited,
                blocks,
                logic_blocks,
            ) {
                return Some(group_id);
            }
        }
        None
    }

    pub fn get_wire_group(
        &self,
        coords: BlockCoordinate,
        wire_color_id: u16,
        logic_block: &LogicBlock,
        structure: &Structure,
        events_by_coords: &HashMap<BlockCoordinate, BlockChangedEvent>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) -> usize {
        self.groups
            .iter()
            .find_map(|(&id, group)| {
                if group.wire_color_id == Some(wire_color_id) && group.recent_wire_coords == Some(coords) {
                    Some(id)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                self.group_dfs_all_faces(
                    logic_block,
                    wire_color_id,
                    coords,
                    structure,
                    events_by_coords,
                    &mut Port::all_for(coords),
                    blocks,
                    logic_blocks,
                )
                .unwrap_or_else(|| {
                    panic!(
                        "Logic block with wire connections (color {}) is not part of any logic group.",
                        wire_color_id
                    )
                })
            })
    }

    pub fn set_group_recent_wire(&mut self, group_id: usize, wire_color_id: u16, coords: BlockCoordinate) {
        let group = self
            .groups
            .get_mut(&group_id)
            .expect("Logic group to update recent wire coordinates should exist.");
        group.recent_wire_coords = Some(coords);
        group.wire_color_id = Some(wire_color_id);
    }

    pub fn add_port(
        &mut self,
        coords: BlockCoordinate,
        direction: BlockDirection,
        group_id: usize,
        port_type: PortType,
        signal: i32,
        entity: Entity,
        evw_queue_logic_output: &mut EventWriter<QueueLogicOutputEvent>,
        evw_queue_logic_input: &mut EventWriter<QueueLogicInputEvent>,
    ) {
        match port_type {
            PortType::Input => &mut self.input_port_group_id,
            PortType::Output => &mut self.output_port_group_id,
        }
        .insert(Port::new(coords, direction), group_id);

        let logic_group = &mut self
            .groups
            .get_mut(&group_id)
            .expect("Group should have vectors of input and output ports.");
        match port_type {
            PortType::Input => {
                logic_group.consumers.insert(Port::new(coords, direction));
                evw_queue_logic_input.send(QueueLogicInputEvent::new(StructureBlock::new(coords, entity)));
            }
            PortType::Output => {
                logic_group.producers.insert(Port::new(coords, direction), signal);
                evw_queue_logic_output.send(QueueLogicOutputEvent::new(StructureBlock::new(coords, entity)));
            }
        };
    }

    pub fn remove_port(
        &mut self,
        coords: BlockCoordinate,
        direction: BlockDirection,
        port_type: PortType,
        structure: &Structure,
        events_by_coords: &HashMap<BlockCoordinate, BlockChangedEvent>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
        evw_queue_logic_input: &mut EventWriter<QueueLogicInputEvent>,
    ) {
        // If the neighbor coordinates don't exist, no port is removed.
        let Ok(neighbor_coords) = coords.step(direction) else {
            return;
        };

        let port = Port::new(coords, direction);
        let &group_id = match port_type {
            PortType::Input => &mut self.input_port_group_id,
            PortType::Output => &mut self.output_port_group_id,
        }
        .get(&port)
        .expect("Port to be removed should exist.");

        // Check if this port is the last block of its group, and delete the group if so.
        if self
            .dfs_for_group(
                neighbor_coords,
                direction.inverse(),
                None,
                false,
                structure,
                events_by_coords,
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
                    evw_queue_logic_input.send(QueueLogicInputEvent::new(StructureBlock::new(
                        input_port.coords,
                        structure.get_entity().expect("Structure should have entity."),
                    )));
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
        wire_color_id: u16,
        group_ids: &HashSet<usize>,
        coords: BlockCoordinate,
        entity: Entity,
        evw_queue_logic_input: &mut EventWriter<QueueLogicInputEvent>,
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
        self.groups.insert(
            new_group_id,
            LogicGroup::new_with_ports(Some(wire_color_id), Some(coords), producers, consumers),
        );

        // Notify all the input ports in the new group that their group's value may have changed.
        for &input_port in self
            .groups
            .get(&new_group_id)
            .expect("Merged logic group should exist.")
            .consumers
            .iter()
        {
            evw_queue_logic_input.send(QueueLogicInputEvent::new(StructureBlock::new(input_port.coords, entity)));
        }
    }

    /// Explores a logic group using DFS, renaming any ports encountered with a new group ID.
    /// Returns whether the new group ID passed in was used (true), or should be deleted (false).
    pub fn rename_group(
        &mut self,
        new_group_id: usize,
        coords: BlockCoordinate,
        encountered_from_direction: BlockDirection,
        wire_color_id: u16,
        from_bus: bool,
        structure: &Structure,
        events_by_coords: &HashMap<BlockCoordinate, BlockChangedEvent>,
        visited: &mut HashSet<Port>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
        evw_queue_logic_output: &mut EventWriter<QueueLogicOutputEvent>,
        evw_queue_logic_input: &mut EventWriter<QueueLogicInputEvent>,
    ) -> bool {
        if visited.contains(&Port::new(coords, encountered_from_direction)) {
            // Renaming on this portion already completed.
            return false;
        }
        let block = self.block_at(coords, structure, events_by_coords, blocks);
        let Some(logic_block) = logic_blocks.from_id(block.unlocalized_name()) else {
            // Not a logic block.
            return false;
        };

        // Return true (the group ID was used) if this block connects to the original block.
        // This recursive function does not pass it's return value between calls. This return only cares about the first block.
        let encountered_face = structure.block_rotation(coords).block_face_pointing(encountered_from_direction);
        match logic_block.connection_on(encountered_face) {
            Some(LogicConnection::Port(port_type)) => {
                // Logic buses do not interact with input/output ports.
                if from_bus {
                    false
                } else {
                    // Getting the port's output value in the previous group.
                    let old_signal = match port_type {
                        PortType::Input => 0,
                        PortType::Output => {
                            let old_group = self
                                .group_of(&Port::new(coords, encountered_from_direction), PortType::Output)
                                .expect("Port being renamed should have a previous group.");
                            *old_group
                                .producers
                                .get(&Port::new(coords, encountered_from_direction))
                                .expect("Existing output port should be either on or off")
                        }
                    };

                    // Inserting the port into the port to group ID mapping also removes the old version.
                    self.add_port(
                        coords,
                        encountered_from_direction,
                        new_group_id,
                        port_type,
                        old_signal,
                        structure.get_entity().expect("Structure should have entity"),
                        evw_queue_logic_output,
                        evw_queue_logic_input,
                    );
                    true
                }
            }
            Some(LogicConnection::Wire(wire_type)) => {
                if wire_type.connects_to_color(wire_color_id) {
                    // Recurse to continue marking the ports reachable from this wire.
                    visited.insert(Port::new(coords, encountered_from_direction));
                    for face in logic_block.wire_faces_connecting_to(wire_type) {
                        let direction = structure.block_rotation(coords).direction_of(face);
                        visited.insert(Port::new(coords, direction));
                        let Ok(neighbor_coords) = coords.step(direction) else {
                            continue;
                        };
                        if visited.contains(&Port::new(neighbor_coords, direction.inverse())) {
                            continue;
                        }
                        self.rename_group(
                            new_group_id,
                            neighbor_coords,
                            direction.inverse(),
                            wire_color_id,
                            wire_type == WireType::Bus,
                            structure,
                            visited,
                            blocks,
                            logic_blocks,
                            evw_queue_logic_output,
                            evw_queue_logic_input,
                        );
                    }
                    // The first wire coords are always set last (so they take effect), the only recursive call is in this arm.
                    self.set_group_recent_wire(new_group_id, wire_color_id, coords);
                    true
                } else {
                    false
                }
            }
            None => false,
        }
    }

    pub fn update_producer(
        &mut self,
        port: Port,
        signal: i32,
        evw_queue_logic_input: &mut EventWriter<QueueLogicInputEvent>,
        entity: Entity,
    ) {
        self.mut_group_of(&port, PortType::Output)
            .expect("Updated logic port should have a logic group ID.")
            .update_producer(port, signal, evw_queue_logic_input, entity);
    }
}
