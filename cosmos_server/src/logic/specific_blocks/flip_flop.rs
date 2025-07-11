//! Logic behavior for the switch, a block that outputs a logic signal on all 6 faces when on.

use std::{cell::RefCell, rc::Rc};

use bevy::prelude::*;

use cosmos_core::{
    block::{Block, block_face::BlockFace, data::BlockData},
    events::block_events::BlockDataSystemParams,
    logic::BlockLogicData,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::Structure,
};

use crate::logic::{
    LOGIC_BIT, LogicBlock, LogicConnection, LogicInputEvent, LogicOutputEvent, LogicSystemSet, PortType, QueueLogicInputEvent,
    default_logic_block_output, logic_driver::LogicDriver,
};

const BLOCK_ID: &str = "cosmos:flip_flop";

fn register_logic_connections(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(block) = blocks.from_id(BLOCK_ID) {
        registry.register(LogicBlock::new(
            block,
            [
                Some(LogicConnection::Port(PortType::Output)),
                Some(LogicConnection::Port(PortType::Output)),
                Some(LogicConnection::Port(PortType::Output)),
                Some(LogicConnection::Port(PortType::Output)),
                Some(LogicConnection::Port(PortType::Input)),
                Some(LogicConnection::Port(PortType::Output)),
            ],
        ));
    }
}

const LAST_STATE_BIT: u8 = LOGIC_BIT >> 1;

fn flip_flop_input_event_listener(
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
        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != BLOCK_ID {
            continue;
        }
        let Ok(logic_driver) = q_logic_driver.get_mut(ev.block.structure()) else {
            continue;
        };
        let mut logic_data = structure
            .query_block_data(ev.block.coords(), &mut q_logic_data)
            .copied()
            .unwrap_or_default();

        let coords = ev.block.coords();
        let rotation = structure.block_rotation(ev.block.coords());
        let input = logic_driver.read_input(coords, rotation.direction_of(BlockFace::Front));
        let new_state = BlockLogicData(input);

        let mut data = structure.block_info_at(ev.block.coords());
        let og_data = data;

        if data.0 & LAST_STATE_BIT == 0 {
            // last state was off

            if new_state.0 != 0 {
                data.0 |= LAST_STATE_BIT;
                data.0 ^= LOGIC_BIT;
                if data.0 & LOGIC_BIT == 0 {
                    logic_data = BlockLogicData::default();
                } else {
                    logic_data = new_state;
                }
            }
        } else {
            // last state was on
            if new_state.0 == 0 {
                data.0 &= !LAST_STATE_BIT;
            }
        }

        if let Some(mut old_data) = structure.query_block_data_mut(ev.block.coords(), &mut q_logic_data, bs_params.clone()) {
            if **old_data != new_state {
                **old_data = new_state;
            }
        } else if logic_data.0 != 0 {
            structure.insert_block_data(coords, logic_data, &mut bs_params.borrow_mut(), &mut q_block_data, &q_has_data);
        }

        if og_data != data {
            structure.set_block_info_at(ev.block.coords(), data, &mut bs_params.borrow_mut().ev_writer);
        }
    }
}

fn flip_flop_output_event_listener(
    evr_logic_output: EventReader<LogicOutputEvent>,
    evw_queue_logic_input: EventWriter<QueueLogicInputEvent>,
    logic_blocks: Res<Registry<LogicBlock>>,
    blocks: Res<Registry<Block>>,
    q_structure: Query<(&mut Structure, &mut LogicDriver)>,
    q_logic_data: Query<&BlockLogicData>,
) {
    default_logic_block_output(
        BLOCK_ID,
        evr_logic_output,
        evw_queue_logic_input,
        &logic_blocks,
        &blocks,
        q_structure,
        q_logic_data,
    );
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), register_logic_connections)
        .add_systems(
            FixedUpdate,
            flip_flop_input_event_listener
                .in_set(LogicSystemSet::Consume)
                .ambiguous_with(LogicSystemSet::Consume),
        )
        .add_systems(
            FixedUpdate,
            flip_flop_output_event_listener
                .in_set(LogicSystemSet::Produce)
                .ambiguous_with(LogicSystemSet::Produce),
        );
}
