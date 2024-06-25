use bevy::{
    app::{App, Update},
    prelude::{in_state, Added, Deref, DerefMut, OnEnter, ResMut, States},
    prelude::{Commands, Component, Entity, EventReader, IntoSystemConfigs, Query, Res, With, Without},
    reflect::Reflect,
    time::Time,
    utils::HashSet,
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

    /// Returns a vector of all block faces with the specified port type - for example: input or output.
    pub fn faces_with<'a>(&'a self, port_type: PortType) -> impl Iterator<Item = BlockFace> + 'a {
        self.ports
            .iter()
            .enumerate()
            .flat_map(|(idx, maybe_port)| maybe_port.map(|port| (BlockFace::from_index(idx), port)))
            .filter(move |(_, port)| *port == port_type)
            .map(|(x, _)| x)
    }

    // Any block with 6 connections and no input/output is a wire.
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

#[derive(Debug, Default, Reflect)]
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
    group: u32,
}

impl Port {
    fn new(coords: BlockCoordinate, group: u32) -> Port {
        Port { coords, group }
    }
}

#[derive(Debug, Default, Reflect, Component)]
struct WireGraph {
    groups: Vec<LogicGroup>,
    producers: Vec<Port>,
    consumers: Vec<Port>,
}

impl WireGraph {
    fn add_group(&mut self, on: bool, coords: BlockCoordinate) {
        self.groups.push(LogicGroup::new(on, coords));
    }

    fn remove_group(&mut self) {}

    fn merge_adjacent_groups(&self, coords: BlockCoordinate, structure: &Structure) {}

    fn dfs_find_group(
        &self,
        coords: BlockCoordinate,
        structure: &Structure,
        start_face: BlockFace,
        visited: &mut HashSet<BlockCoordinate>,
        blocks: &Registry<Block>,
        logic_blocks: &Registry<LogicBlock>,
    ) -> Option<u32> {
        let block = structure.block_at(coords, blocks);
        let Some(logic_block) = logic_blocks.from_id(block.unlocalized_name()) else {
            // Not a logic block.
            return None;
        };

        // Input port found, return the group the port is part of.
        if logic_block.faces_with(PortType::Input).any(|face| face == start_face) {
            if let Some(port) = self.consumers.iter().find(|gate| gate.coords == coords) {
                return Some(port.group);
            }
        }

        // Output gate found, retrun the group the port is part of.
        if logic_block.faces_with(PortType::Output).any(|face| face == start_face) {
            if let Some(port) = self.producers.iter().find(|gate| gate.coords == coords) {
                return Some(port.group);
            }
        }

        // Most recent wire in the logic group found, return its group.
        if logic_block.is_wire() {
            if let Some(group) = self.groups.iter().position(|group| group.recent_coords == coords) {
                return Some(group as u32);
            }
        }

        // This block does not tell us what group we're in. Recurse on its neighbors.
        visited.insert(coords);
        for face in logic_block.faces_with(PortType::Wire) {
            let Ok(neighbor_coords) = coords.step(face) else {
                continue;
            };

            if !visited.contains(&neighbor_coords) {
                if let Some(group) = self.dfs_find_group(neighbor_coords, structure, face.inverse(), visited, blocks, logic_blocks) {
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
