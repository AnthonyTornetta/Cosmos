//! Logic behavior for "Logic Indicator", a block that changes its appearance to indicate if any of its 6 input ports are recieving logic "on".

use std::{cell::RefCell, rc::Rc};

use bevy::prelude::*;

use cosmos_core::{
    block::Block,
    events::block_events::BlockDataChangedEvent,
    logic::{BlockLogicData, HasOnOffInfo},
    registry::{Registry, identifiable::Identifiable},
    structure::Structure,
};

use crate::logic::{LogicBlock, LogicConnection, LogicInputEvent, LogicSystemSet, PortType, logic_driver::LogicDriver};

fn register_logic_ports(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(logic_indicator) = blocks.from_id("cosmos:logic_indicator") {
        registry.register(LogicBlock::new(logic_indicator, [Some(LogicConnection::Port(PortType::Input)); 6]));
    }
}

fn logic_indicator_input_event_listener(
    mut evr_logic_input: EventReader<LogicInputEvent>,
    blocks: Res<Registry<Block>>,
    mut q_structure: Query<(&mut Structure, &LogicDriver)>,
    mut evw_block_data_changed: EventWriter<BlockDataChangedEvent>,
) {
    for ev in evr_logic_input.read() {
        let Ok((mut structure, logic_driver)) = q_structure.get_mut(ev.block.structure()) else {
            continue;
        };
        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:logic_indicator" {
            continue;
        }

        let new_state = BlockLogicData(
            logic_driver
                .read_all_inputs(ev.block.coords(), structure.block_rotation(ev.block.coords()))
                .iter()
                .any(|signal| *signal != 0) as i32,
        );

        let cur_info = structure.block_info_at(ev.block.coords());
        let mut new_info = cur_info;
        if new_state.on() {
            new_info.set_on();
        } else {
            new_info.set_off();
        }

        if cur_info != new_info {
            structure.set_block_info_at(ev.block.coords(), new_info, &mut evw_block_data_changed);
        }
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_ports).add_systems(
        FixedUpdate,
        logic_indicator_input_event_listener
            .in_set(LogicSystemSet::Consume)
            .ambiguous_with(LogicSystemSet::Consume),
    );
}
