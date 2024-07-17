//! Logic behavior for "Or Gate", a block with left and right inputs and a front output.
//! Outputs 0 if both inputs are zero or missing. Outputs 1 if either input is present and non-zero.

use std::{cell::RefCell, rc::Rc};

use bevy::{
    app::{App, Update},
    prelude::{EventReader, EventWriter, IntoSystemConfigs, OnEnter, Query, Res, ResMut, States},
};

use crate::{
    block::{Block, BlockFace},
    events::block_events::BlockDataSystemParams,
    logic::{
        logic_driver::LogicDriver, BlockLogicData, LogicBlock, LogicConnection, LogicInputEvent, LogicOutputEvent, LogicSystemSet, Port,
        PortType, QueueLogicInputEvent,
    },
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};

fn register_logic_connections(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(and_gate) = blocks.from_id("cosmos:or_gate") {
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
            0,
        ));
    }
}

fn or_gate_input_event_listener(
    mut evr_logic_input: EventReader<LogicInputEvent>,
    mut evw_logic_output: EventWriter<LogicOutputEvent>,
    blocks: Res<Registry<Block>>,
    mut q_logic_driver: Query<&mut LogicDriver>,
    q_structure: Query<&Structure>,
    mut q_logic_data: Query<&mut BlockLogicData>,
    bs_params: BlockDataSystemParams,
) {
    let bs_params = Rc::new(RefCell::new(bs_params));
    for ev in evr_logic_input.read() {
        let Ok(structure) = q_structure.get(ev.entity) else {
            continue;
        };
        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:or_gate" {
            continue;
        }
        let Ok(logic_driver) = q_logic_driver.get_mut(ev.entity) else {
            continue;
        };
        let Some(mut logic_data) = structure.query_block_data_mut(ev.block.coords(), &mut q_logic_data, bs_params.clone()) else {
            continue;
        };

        let coords = ev.block.coords();
        let rotation = structure.block_rotation(ev.block.coords());
        let left = logic_driver.read_input(coords, rotation.direction_of(BlockFace::Left)) != 0;
        let right = logic_driver.read_input(coords, rotation.direction_of(BlockFace::Right)) != 0;
        let new_state = BlockLogicData((left || right) as i32);

        if **logic_data != new_state {
            // Don't trigger unneccesary change detection
            **logic_data = new_state;
            evw_logic_output.send(LogicOutputEvent {
                block: ev.block,
                entity: ev.entity,
            });
        }
    }
}

fn or_gate_output_event_listener(
    mut evr_logic_output: EventReader<LogicOutputEvent>,
    mut evw_logic_input: EventWriter<QueueLogicInputEvent>,
    blocks: Res<Registry<Block>>,
    mut q_logic_driver: Query<&mut LogicDriver>,
    mut q_structure: Query<&mut Structure>,
    q_logic_data: Query<&BlockLogicData>,
) {
    for ev in evr_logic_output.read() {
        let Ok(structure) = q_structure.get_mut(ev.entity) else {
            continue;
        };
        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:or_gate" {
            continue;
        }
        let Ok(mut logic_driver) = q_logic_driver.get_mut(ev.entity) else {
            continue;
        };
        let Some(&BlockLogicData(signal)) = structure.query_block_data(ev.block.coords(), &q_logic_data) else {
            continue;
        };

        let port = Port::new(
            ev.block.coords(),
            structure.block_rotation(ev.block.coords()).direction_of(BlockFace::Front),
        );
        logic_driver.update_producer(port, signal, &mut evw_logic_input, ev.entity);
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_connections)
        .add_systems(Update, or_gate_input_event_listener.in_set(LogicSystemSet::Consume))
        .add_systems(Update, or_gate_output_event_listener.in_set(LogicSystemSet::Produce));
}
