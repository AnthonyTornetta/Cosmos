//! Public interface for controlling the behavior of the logic system, which involves all logic blocks in an entity.

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use serde::{Deserialize, Serialize};

use cosmos_core::{
    block::{Block, block_direction::BlockDirection, block_face::ALL_BLOCK_FACES, block_rotation::BlockRotation},
    netty::sync::IdentifiableComponent,
    registry::Registry,
    structure::{Structure, coordinates::BlockCoordinate, structure_block::StructureBlock},
};

use crate::{logic::LogicConnection, persistence::make_persistent::DefaultPersistentComponent};

use super::{LogicBlock, LogicWireColor, Port, PortType, QueueLogicInputMessage, QueueLogicOutputMessage, WireType, logic_graph::LogicGraph};

#[derive(Debug, Default, Reflect, Component, Serialize, Deserialize, Clone, PartialEq)]
/// The public interface for accessing and mutating an [`Entity`]'s [`LogicGraph`].
///
/// Any functionality needed for specific logic blocks (for example, wires and logic gates) should use this struct and never directly access the [`LogicGraph`].
pub struct LogicDriver {
    logic_graph: LogicGraph,
}

impl IdentifiableComponent for LogicDriver {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:logic_driver"
    }
}

impl DefaultPersistentComponent for LogicDriver {}

#[derive(Clone, Copy, Debug)]
pub(super) struct LogicBlockChangedMessage<'a> {
    pub coord: BlockCoordinate,
    pub old: Option<(&'a LogicBlock, BlockRotation)>,
    pub new: Option<(&'a LogicBlock, BlockRotation)>,
}

impl LogicDriver {
    /// Returns an array of the Boolean value of the given block's input port groups.
    /// A block face without an input port is assigned `0`.
    pub fn read_input(&self, coords: BlockCoordinate, direction: BlockDirection) -> i32 {
        self.logic_graph
            .group_of(&Port::new(coords, direction), PortType::Input)
            .map(|group| group.signal())
            .unwrap_or(0)
    }

    /// Gets the input signals of all 6 faces, in the order of the [`Direction`] indices.
    pub fn read_all_inputs(&self, coords: BlockCoordinate, rotation: BlockRotation) -> [i32; 6] {
        ALL_BLOCK_FACES.map(|face| self.read_input(coords, rotation.direction_of(face)))
    }

    fn port_placed(
        &mut self,
        coords: BlockCoordinate,
        direction: BlockDirection,
        port_type: PortType,
        structure: &Structure,
        entity: Entity,
        events_by_coords: &HashMap<BlockCoordinate, LogicBlockChangedMessage>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
        evw_queue_logic_output: &mut MessageWriter<QueueLogicOutputMessage>,
        evw_queue_logic_input: &mut MessageWriter<QueueLogicInputMessage>,
    ) {
        // If the neighbor coordinates don't exist, no port is added (and thus no new group).
        let Ok(neighbor_coords) = coords.step(direction) else {
            return;
        };

        // DFS on the structure may lead to concurrency issues if many blocks are changed in one tick.
        // See [`LogicGraph::remove_port`] and how it removed dfs.
        let maybe_group = self.logic_graph.dfs_for_group(
            neighbor_coords,
            direction.inverse(),
            None,
            false,
            structure,
            events_by_coords,
            &mut Port::all_for(coords),
            blocks,
            logic_blocks,
        );
        let group_id = maybe_group.unwrap_or_else(|| self.logic_graph.new_group(None, None));
        self.logic_graph.add_port(
            coords,
            direction,
            group_id,
            port_type,
            0,
            entity,
            evw_queue_logic_output,
            evw_queue_logic_input,
        );
    }

    /// Adds a logic block, along with all of its ports and wire connections, to the graph.
    /// If the added block has wire connections, merges adjacent [`LogicGroup`]s into a single group.
    pub(super) fn add_logic_block(
        &mut self,
        logic_block: &LogicBlock,
        rotation: BlockRotation,
        coords: BlockCoordinate,
        structure: &Structure,
        entity: Entity,
        events_by_coords: &HashMap<BlockCoordinate, LogicBlockChangedMessage>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
        logic_wire_colors: &Registry<LogicWireColor>,
        evw_queue_logic_output: &mut MessageWriter<QueueLogicOutputMessage>,
        evw_queue_logic_input: &mut MessageWriter<QueueLogicInputMessage>,
    ) {
        // Adding input faces as consumers to their connected group, or a new group if there is no connected group.
        for input_face in logic_block.input_faces() {
            self.port_placed(
                coords,
                rotation.direction_of(input_face),
                PortType::Input,
                structure,
                entity,
                events_by_coords,
                blocks,
                logic_blocks,
                evw_queue_logic_output,
                evw_queue_logic_input,
            )
        }

        // Adding output faces as producers to their connected group, or a new group if there is no connected group.
        for output_face in logic_block.output_faces() {
            self.port_placed(
                coords,
                rotation.direction_of(output_face),
                PortType::Output,
                structure,
                entity,
                events_by_coords,
                blocks,
                logic_blocks,
                evw_queue_logic_output,
                evw_queue_logic_input,
            )
        }

        // A block with no wire faces will have an empty wire face colors iterator.
        for wire_color_id in logic_block.wire_face_colors(logic_wire_colors) {
            // Connect wire faces to all existing groups (by creating one new group that includes all adjacent groups).
            let mut group_ids: HashSet<usize> = HashSet::new();

            // Get all adjacent group IDs.
            for wire_face in logic_block.wire_faces_connecting_to(WireType::Color(wire_color_id)) {
                let direction = structure.block_rotation(coords).direction_of(wire_face);
                if let Ok(neighbor_coords) = coords.step(direction) {
                    // DFS on the structure may lead to concurrency issues if many blocks are changed in one tick.
                    // See [`LogicGraph::remove_port`] and how it removed dfs.
                    if let Some(group_id) = self.logic_graph.dfs_for_group(
                        neighbor_coords,
                        direction.inverse(),
                        Some(wire_color_id),
                        logic_block.connection_on(wire_face) == Some(crate::logic::LogicConnection::Wire(WireType::Bus)),
                        structure,
                        events_by_coords,
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
                0 => drop(self.logic_graph.new_group(Some(wire_color_id), Some(coords))),
                1 => self
                    .logic_graph
                    .set_group_recent_wire(*group_ids.iter().next().unwrap(), wire_color_id, coords),
                _ => self
                    .logic_graph
                    .merge_adjacent_groups(wire_color_id, &group_ids, coords, entity, evw_queue_logic_input),
            };
        }
    }

    /// Removes a logic block, along with all of its ports and wire connections, from the graph.
    /// If the removed block has wire connections, might split its [`LogicGroup`] into several disconnected groups.
    pub(super) fn remove_logic_block(
        &mut self,
        logic_block: &LogicBlock,
        rotation: BlockRotation,
        coords: BlockCoordinate,
        structure: &Structure,
        entity: Entity,
        events_by_coords: &HashMap<BlockCoordinate, LogicBlockChangedMessage>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
        logic_wire_colors: &Registry<LogicWireColor>,
        evw_queue_logic_output: &mut MessageWriter<QueueLogicOutputMessage>,
        evw_queue_logic_input: &mut MessageWriter<QueueLogicInputMessage>,
    ) {
        // Removing input ports from their groups.
        for input_face in logic_block.input_faces() {
            self.logic_graph.remove_port(
                coords,
                rotation.direction_of(input_face),
                PortType::Input,
                structure,
                events_by_coords,
                blocks,
                logic_blocks,
                evw_queue_logic_input,
            )
        }

        // Removing output ports from their groups.
        for output_face in logic_block.output_faces() {
            self.logic_graph.remove_port(
                coords,
                rotation.direction_of(output_face),
                PortType::Output,
                structure,
                events_by_coords,
                blocks,
                logic_blocks,
                evw_queue_logic_input,
            )
        }

        // A block with no wire faces will have an empty wire face colors iterator.
        for wire_color_id in logic_block.wire_face_colors(logic_wire_colors) {
            // Old group ID either comes from being the stored wire coordinate for a group, or searching all your neighbors.
            let old_group_id = self.logic_graph.get_wire_group(
                coords,
                wire_color_id,
                logic_block,
                structure,
                events_by_coords,
                blocks,
                logic_blocks,
            );
            let was_on = self.logic_graph.get_group(old_group_id).on();

            // Setting new group IDs.
            let mut visited = Port::all_for(coords);
            for wire_face in logic_block.wire_faces_connecting_to(WireType::Color(wire_color_id)) {
                let direction = structure.block_rotation(coords).direction_of(wire_face);
                let Ok(neighbor_coords) = coords.step(direction) else {
                    continue;
                };
                // For now, takes a new ID for every call, even though some (like air blocks or already visited wires) don't need it.
                let group_id = self.logic_graph.new_group(None, None);
                let used_new_group = self.logic_graph.rename_group(
                    group_id,
                    neighbor_coords,
                    direction.inverse(),
                    wire_color_id,
                    logic_block.connection_on(wire_face) == Some(LogicConnection::Wire(WireType::Bus)),
                    structure,
                    events_by_coords,
                    &mut visited,
                    blocks,
                    logic_blocks,
                    evw_queue_logic_output,
                    evw_queue_logic_input,
                );
                if !used_new_group {
                    self.logic_graph.remove_group(group_id);
                } else {
                    let new_group = self.logic_graph.get_group(group_id);
                    if new_group.on() != was_on {
                        // Update the inputs to every input port in this newly created group, if the value of the group has changed.
                        evw_queue_logic_input.write_batch(
                            new_group
                                .consumers
                                .iter()
                                .map(|input_port| QueueLogicInputMessage::new(StructureBlock::new(input_port.coords, entity))),
                        );
                    }
                }
            }
            self.logic_graph.remove_group(old_group_id);
        }
    }

    /// Sets the on/off value of the given port (which must be an output port) in the logic graph.
    pub fn update_producer(
        &mut self,
        port: Port,
        signal: i32,
        evw_queue_logic_input: &mut MessageWriter<QueueLogicInputMessage>,
        entity: Entity,
    ) {
        self.logic_graph.update_producer(port, signal, evw_queue_logic_input, entity);
    }
}
