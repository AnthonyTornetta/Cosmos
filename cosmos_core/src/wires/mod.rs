use bevy::{
    app::{App, Update},
    prelude::{
        in_state, Added, Commands, Component, Deref, DerefMut, Entity, EventReader, IntoSystemConfigs, OnEnter, Query, Res, ResMut, States,
        With, Without,
    },
    reflect::Reflect,
    time::Time,
    utils::{HashMap, HashSet},
};

use crate::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{
        coordinates::{BlockCoordinate, UnboundBlockCoordinate},
        loading::StructureLoadingSet,
        structure_block::StructureBlock,
        systems::{energy_storage_system::EnergyStorageSystem, StructureSystems},
        Structure,
    },
};

fn wire_place_event_listener(
    mut evr_block_updated: EventReader<BlockChangedEvent>,
    registry: Res<Registry<Block>>,
    mut q_wire_graph: Query<&mut WireGraph>,
    mut q_structure: Query<&mut Structure>,
) {
    let Some(wire_block) = registry.from_id("cosmos:logic_wire") else {
        return;
    };

    let Some(logic_on) = registry.from_id("cosmos:logic_on") else {
        return;
    };

    // ev.block.coords
    // structure.block_info_at(BlockCoordinate)
    // structure.block_rotation(BlockCoordinate).local_front().direction_coordinates()

    for ev in evr_block_updated.read() {
        // If was wire, remove from graph.
        if ev.old_block == wire_block.id() {
            let Ok(mut wire_graph) = q_wire_graph.get_mut(ev.structure_entity) else {
                continue;
            };
        }

        // If is now wire, add to graph.
        if ev.new_block == wire_block.id() {
            let Ok(mut wire_graph) = q_wire_graph.get_mut(ev.structure_entity) else {
                continue;
            };

            let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
                continue;
            };

            // ev.block.coords().front()
            // structure.
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PortType {
    // Reads the Boolean value of the logic group adjacent to this face to help compute its internal Boolean value.
    Input,
    // Writes its internal Boolean value to the logic group adjacent to this face.
    Output,
    // Part of any adjacent logic groups without interrupting them or having delayed inputs or outputs - for example: wires.
    Wire,
}

#[derive(Debug, Clone)]
/// A block that interacts with the logic system, like wires and gates.
pub struct LogicBlock {
    // Specifies the roles of the 6 block faces, ordered by BlockFace index.
    ports: [Option<PortType>; 6],

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
    pub fn new(block: &Block, ports: [Option<PortType>; 6]) -> Self {
        Self {
            ports,
            id: 0,
            unlocalized_name: block.unlocalized_name().to_owned(),
        }
    }

    /// Returns an iterator over all block faces with any port.
    pub fn faces<'a>(&'a self) -> impl Iterator<Item = BlockFace> + 'a {
        self.ports
            .iter()
            .enumerate()
            .filter(|(_, maybe_port)| maybe_port.is_some())
            .map(|(idx, _)| BlockFace::from_index(idx))
    }

    /// Returns an iterator over all block faces with the specified port type - for example: input or output.
    pub fn faces_with<'a>(&'a self, port_type: PortType) -> impl Iterator<Item = BlockFace> + 'a {
        self.ports
            .iter()
            .enumerate()
            .filter(move |(_, maybe_port)| **maybe_port == Some(port_type))
            .map(|(idx, _)| BlockFace::from_index(idx))
    }

    /// Any block with 6 connections and no input/output is a wire.
    pub fn is_wire(&self) -> bool {
        self.faces_with(PortType::Wire).count() == 6
    }
}

fn register_logic_blocks(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    use PortType as FP;
    if let Some(logic_wire) = blocks.from_id("cosmos:logic_wire") {
        registry.register(LogicBlock::new(logic_wire, [Some(FP::Wire); 6]));
    }
    if let Some(logic_on) = blocks.from_id("cosmos:logic_on") {
        registry.register(LogicBlock::new(logic_on, [Some(FP::Output); 6]));
    }
    if let Some(light) = blocks.from_id("cosmos:light") {
        registry.register(LogicBlock::new(light, [Some(FP::Input); 6]));
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
    recent_coords: BlockCoordinate,
}

impl LogicGroup {
    fn new(on: bool, coords: BlockCoordinate) -> LogicGroup {
        LogicGroup { on, recent_coords: coords }
    }
}

#[derive(Debug, Default, Reflect)]
struct Port {
    coords: BlockCoordinate,
    group_id: usize,
}

impl Port {
    fn new(coords: BlockCoordinate, group: usize) -> Port {
        Port { coords, group_id: group }
    }
}

#[derive(Debug, Default, Reflect, Component)]
struct WireGraph {
    /// As new logic groups are created, this tracks which ID is the next available.
    next_group_id: usize,
    groups: HashMap<usize, LogicGroup>,
    outputs: Vec<Port>,
    inputs: Vec<Port>,
}

impl WireGraph {
    fn add_group(&mut self, id: usize, on: bool, coords: BlockCoordinate) {
        self.groups.insert(id, LogicGroup::new(on, coords));
    }

    fn remove_group(&mut self) {}

    fn merge_adjacent_groups(
        &mut self,
        coords: BlockCoordinate,
        structure: &Structure,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) {
        let block = structure.block_at(coords, blocks);
        let Some(logic_block) = logic_blocks.from_id(block.unlocalized_name()) else {
            // Not a logic block.
            return;
        };

        // Get all adjacent group ID numbers.
        let mut group_ids: HashSet<usize> = HashSet::new();
        for face in logic_block.faces_with(PortType::Wire) {
            let mut visited: HashSet<BlockCoordinate> = HashSet::new();
            let Ok(neighbor_coords) = coords.step(face) else {
                continue;
            };

            if let Some(group_id) = self.find_group(neighbor_coords, structure, face.inverse(), &mut visited, blocks, logic_blocks) {
                group_ids.insert(group_id);
            }
        }

        let mut new_group_on = false;

        if !group_ids.is_empty() {
            // Rewrite all inputs and outputs of adjacent groups to use the new ID number.
            for output in self.outputs.iter_mut() {
                if group_ids.contains(&output.group_id) {
                    output.group_id = self.next_group_id;
                }
            }

            for input in self.inputs.iter_mut() {
                if group_ids.contains(&input.group_id) {
                    input.group_id = self.next_group_id;
                }
            }

            // The new group is on if any of its neighbors were.
            new_group_on = group_ids.iter().fold(false, |or, group_id| or || self.groups[group_id].on);

            // Remove the old groups.
            for group_id in group_ids {
                self.groups.remove(&group_id);
            }
        }

        // Creating the new group. The most recent block added is the current block.
        self.groups.insert(
            self.next_group_id,
            LogicGroup {
                on: new_group_on,
                recent_coords: coords,
            },
        );

        // Necessary to make sure future groups get a unique ID.
        self.next_group_id += 1;
    }

    fn find_group(
        &self,
        coords: BlockCoordinate,
        structure: &Structure,
        start_face: BlockFace,
        visited: &mut HashSet<BlockCoordinate>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) -> Option<usize> {
        let block = structure.block_at(coords, blocks);
        let Some(logic_block) = logic_blocks.from_id(block.unlocalized_name()) else {
            // Not a logic block.
            return None;
        };

        // Input port found, return the group the port is part of.
        if logic_block.faces_with(PortType::Input).any(|face| face == start_face) {
            if let Some(port) = self.inputs.iter().find(|gate| gate.coords == coords) {
                return Some(port.group_id);
            }
        }

        // Output gate found, retrun the group the port is part of.
        if logic_block.faces_with(PortType::Output).any(|face| face == start_face) {
            if let Some(port) = self.outputs.iter().find(|gate| gate.coords == coords) {
                return Some(port.group_id);
            }
        }

        // Most recent wire in the logic group found, return its group.
        if logic_block.is_wire() {
            if let Some(&id) = self
                .groups
                .iter()
                .find_map(|(id, group)| if group.recent_coords == coords { Some(id) } else { None })
            {
                return Some(id);
            }
        }

        // This block does not tell us what group we're in. Recurse on its neighbors.
        visited.insert(coords);
        for face in logic_block.faces_with(PortType::Wire) {
            let Ok(neighbor_coords) = coords.step(face) else {
                continue;
            };

            if !visited.contains(&neighbor_coords) {
                if let Some(group) = self.find_group(neighbor_coords, structure, face.inverse(), visited, blocks, logic_blocks) {
                    return Some(group);
                }
            }
        }
        return None;
    }
}

fn add_default_wire_graph(q_needs_wire_graph: Query<Entity, (With<Structure>, Without<WireGraph>)>, mut commands: Commands) {
    for entity in q_needs_wire_graph.iter() {
        commands.entity(entity).insert(WireGraph::default());
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    create_registry::<LogicBlock>(app, "cosmos:logic_blocks");

    app.add_systems(OnEnter(post_loading_state), register_logic_blocks)
        .add_systems(Update, add_default_wire_graph.in_set(StructureLoadingSet::AddStructureComponents))
        .register_type::<WireGraph>();
}
