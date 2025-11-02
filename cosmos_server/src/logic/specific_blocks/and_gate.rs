//! Logic behavior for "And Gate", a block with left and right inputs and a front output.
//! Outputs 0 if either input is zero or missing. Outputs 1 if both inputs are present and non-zero.

use std::{cell::RefCell, rc::Rc};

use bevy::prelude::*;

use cosmos_core::{
    block::{Block, block_face::BlockFace, data::BlockData},
    events::block_events::BlockDataSystemParams,
    registry::{Registry, identifiable::Identifiable},
    structure::Structure,
};

use crate::logic::{
    BlockLogicData, LogicBlock, LogicConnection, LogicInputEvent, LogicOutputEvent, LogicSystemSet, PortType, QueueLogicInputEvent,
    default_logic_block_output, logic_driver::LogicDriver,
};

fn register_logic_connections(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(and_gate) = blocks.from_id("cosmos:and_gate") {
        registry.register(LogicBlock::new(
            and_gate,
            [
                Some(LogicConnection::Port(PortType::Input)),
                Some(LogicConnection::Port(PortType::Input)),
                None,
                None,
                Some(LogicConnection::Port(PortType::Output)),
                None,
            ],
        ));
    }
}

fn and_gate_input_event_listener(
    mut evr_logic_input: EventReader<LogicInputEvent>,
    blocks: Res<Registry<Block>>,
    mut q_logic_driver: Query<&mut LogicDriver>,
    mut q_structure: Query<&mut Structure>,
    mut q_logic_data: Query<&mut BlockLogicData>,
    bs_params: BlockDataSystemParams,
    q_has_data: Query<(), With<BlockLogicData>>,
    mut q_block_data: Query<&mut BlockData>,
) {
    let bs_params = Rc::new(RefCell::new(bs_params));
    for ev in evr_logic_input.read() {
        let Ok(mut structure) = q_structure.get_mut(ev.block.structure()) else {
            continue;
        };
        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:and_gate" {
            continue;
        }
        let Ok(logic_driver) = q_logic_driver.get_mut(ev.block.structure()) else {
            continue;
        };

        let coords = ev.block.coords();
        let rotation = structure.block_rotation(ev.block.coords());
        let left = logic_driver.read_input(coords, rotation.direction_of(BlockFace::Left)) != 0;
        let right = logic_driver.read_input(coords, rotation.direction_of(BlockFace::Right)) != 0;
        let new_state = BlockLogicData((left && right) as i32);

        if let Some(mut logic_data) = structure.query_block_data_mut(ev.block.coords(), &mut q_logic_data, bs_params.clone()) {
            if **logic_data != new_state {
                **logic_data = new_state;
            }
        } else if new_state.0 != 0 {
            structure.insert_block_data(coords, new_state, &mut bs_params.borrow_mut(), &mut q_block_data, &q_has_data);
        }
    }
}

fn and_gate_output_event_listener(
    evr_logic_output: EventReader<LogicOutputEvent>,
    evw_queue_logic_input: MessageWriter<QueueLogicInputEvent>,
    logic_blocks: Res<Registry<LogicBlock>>,
    blocks: Res<Registry<Block>>,
    q_structure: Query<(&mut Structure, &mut LogicDriver)>,
    q_logic_data: Query<&BlockLogicData>,
) {
    default_logic_block_output(
        "cosmos:and_gate",
        evr_logic_output,
        evw_queue_logic_input,
        &logic_blocks,
        &blocks,
        q_structure,
        q_logic_data,
    );
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_connections)
        .add_systems(
            FixedUpdate,
            and_gate_input_event_listener
                .in_set(LogicSystemSet::Consume)
                .ambiguous_with(LogicSystemSet::Consume),
        )
        .add_systems(
            FixedUpdate,
            and_gate_output_event_listener
                .in_set(LogicSystemSet::Produce)
                .ambiguous_with(LogicSystemSet::Produce),
        );
}
