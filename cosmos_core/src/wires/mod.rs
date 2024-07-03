use bevy::{
    app::{App, Update},
    prelude::{
        in_state, Commands, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, OnEnter, Query, Res, ResMut, States, With, Without,
    },
};
use wire_graph::{LogicBlock, LogicConnection, LogicGroup, Port, PortType, WireGraph};

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{loading::StructureLoadingSet, Structure},
};

pub mod wire_graph;

fn logic_block_placed_event_listener(
    mut evr_block_updated: EventReader<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    logic_blocks: Res<Registry<LogicBlock>>,
    mut q_wire_graph: Query<&mut WireGraph>,
    mut q_structure: Query<&mut Structure>,
    // mut evw_logic_output: EventWriter<LogicOutputEvent>,
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

#[derive(Event, Debug)]
pub struct LogicOutputEvent {
    pub logic_group_id: usize,
    pub output_port: Port,
    pub entity: Entity,
}

#[derive(Event, Debug)]
pub struct LogicInputEvent {
    pub logic_group_id: usize,
    pub input_port: Port,
    pub entity: Entity,
}

fn logic_output_event_listener(
    mut evr_logic_output: EventReader<LogicOutputEvent>,
    mut evw_logic_input: EventWriter<LogicInputEvent>,
    logic_blocks: Res<Registry<LogicBlock>>,
    mut q_wire_graph: Query<&mut WireGraph>,
    mut q_structure: Query<&mut Structure>,
) {
    for ev in evr_logic_output.read() {
        let Ok(structure) = q_structure.get_mut(ev.entity) else {
            return;
        };
        let Ok(mut wire_graph) = q_wire_graph.get_mut(ev.entity) else {
            return;
        };
    }
    // evw_logic_input.send(LogicInputEvent {  })
}

fn add_default_wire_graph(q_needs_wire_graph: Query<Entity, (With<Structure>, Without<WireGraph>)>, mut commands: Commands) {
    for entity in q_needs_wire_graph.iter() {
        commands.entity(entity).insert(WireGraph::default());
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

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T, playing_state: T) {
    create_registry::<LogicBlock>(app, "cosmos:logic_blocks");

    app.add_systems(OnEnter(post_loading_state), register_logic_blocks)
        .add_systems(
            Update,
            (
                add_default_wire_graph.in_set(StructureLoadingSet::AddStructureComponents),
                logic_block_placed_event_listener,
                // (logic_output_event_listener, light_logic_input_event_listener).chain(),
            )
                .run_if(in_state(playing_state)),
        )
        .register_type::<WireGraph>()
        .register_type::<LogicGroup>()
        .add_event::<LogicOutputEvent>()
        .add_event::<LogicInputEvent>();
}
