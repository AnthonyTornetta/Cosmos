//! Logic behavior for "Or Gate", a block with left and right inputs and a front output.
//! Outputs 0 if both inputs are zero or missing. Outputs 1 if either input is present and non-zero.

use std::{cell::RefCell, rc::Rc};

use bevy::prelude::*;

use cosmos_core::{
    block::{Block, block_face::BlockFace},
    events::block_events::BlockDataSystemParams,
    registry::{Registry, identifiable::Identifiable},
    structure::Structure,
};

use crate::logic::{
    BlockLogicData, LogicBlock, LogicConnection, LogicInputEvent, LogicOutputEvent, LogicSystemSet, PortType, QueueLogicInputEvent,
    default_logic_block_output, logic_driver::LogicDriver,
};

fn register_logic_connections(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(not_gate) = blocks.from_id("cosmos:not_gate") {
        registry.register(LogicBlock::new(
            not_gate,
            [
                None,
                None,
                None,
                None,
                Some(LogicConnection::Port(PortType::Output)),
                Some(LogicConnection::Port(PortType::Input)),
            ],
        ));
    }
}

fn not_gate_input_event_listener(
    mut evr_logic_input: EventReader<LogicInputEvent>,
    blocks: Res<Registry<Block>>,
    mut q_logic_driver: Query<&mut LogicDriver>,
    q_structure: Query<&Structure>,
    mut q_logic_data: Query<&mut BlockLogicData>,
    bs_params: BlockDataSystemParams,
) {
    let bs_params = Rc::new(RefCell::new(bs_params));
    for ev in evr_logic_input.read() {
        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            continue;
        };
        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:not_gate" {
            continue;
        }
        let Ok(logic_driver) = q_logic_driver.get_mut(ev.block.structure()) else {
            continue;
        };
        let Some(mut logic_data) = structure.query_block_data_mut(ev.block.coords(), &mut q_logic_data, bs_params.clone()) else {
            continue;
        };

        let coords = ev.block.coords();
        let rotation = structure.block_rotation(ev.block.coords());
        let input = logic_driver.read_input(coords, rotation.direction_of(BlockFace::Back)) != 0;
        let new_state = BlockLogicData(!input as i32);

        if **logic_data != new_state {
            // Don't trigger unneccesary change detection.
            **logic_data = new_state;
        }
    }
}

fn not_gate_output_event_listener(
    evr_logic_output: EventReader<LogicOutputEvent>,
    evw_queue_logic_input: EventWriter<QueueLogicInputEvent>,
    logic_blocks: Res<Registry<LogicBlock>>,
    blocks: Res<Registry<Block>>,
    q_logic_driver: Query<&mut LogicDriver>,
    q_structure: Query<&mut Structure>,
    q_logic_data: Query<&BlockLogicData>,
) {
    default_logic_block_output(
        "cosmos:not_gate",
        evr_logic_output,
        evw_queue_logic_input,
        &logic_blocks,
        &blocks,
        q_logic_driver,
        q_structure,
        q_logic_data,
    );
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_connections)
        .add_systems(
            FixedUpdate,
            not_gate_input_event_listener
                .in_set(LogicSystemSet::Consume)
                .ambiguous_with(LogicSystemSet::Consume),
        )
        .add_systems(
            FixedUpdate,
            not_gate_output_event_listener
                .in_set(LogicSystemSet::Produce)
                .ambiguous_with(LogicSystemSet::Produce),
        );
}
