//! Logic behavior for "Numeric Display", a block that displays a digit 0-9 representing the logic signal it's recieving.

use std::{cell::RefCell, rc::Rc};

use bevy::prelude::*;

use cosmos_core::{
    block::{Block, block_face::BlockFace, data::BlockData},
    events::block_events::BlockDataSystemParams,
    registry::{Registry, identifiable::Identifiable},
    structure::Structure,
};

use crate::logic::{BlockLogicData, LogicBlock, LogicConnection, LogicInputEvent, LogicSystemSet, PortType, logic_driver::LogicDriver};

fn register_logic_ports(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(numeric_display) = blocks.from_id("cosmos:numeric_display") {
        registry.register(LogicBlock::new(
            numeric_display,
            [None, Some(LogicConnection::Port(PortType::Input)), None, None, None, None],
        ));
    }
}

fn numeric_display_input_event_listener(
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
        let new_state = BlockLogicData(logic_driver.read_input(coords, rotation.direction_of(BlockFace::Left)));

        if let Some(mut logic_data) = structure.query_block_data_mut(ev.block.coords(), &mut q_logic_data, bs_params.clone()) {
            if **logic_data != new_state {
                **logic_data = new_state;
            }
        } else if new_state.0 != 0 {
            structure.insert_block_data(coords, new_state, &mut bs_params.borrow_mut(), &mut q_block_data, &q_has_data);
        }
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_ports).add_systems(
        FixedUpdate,
        numeric_display_input_event_listener
            .in_set(LogicSystemSet::Consume)
            .ambiguous_with(LogicSystemSet::Consume),
    );
}
