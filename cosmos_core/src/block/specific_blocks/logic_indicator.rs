//! Logic behavior for "Logic Indicator", a block that changes its appearance to indicate if any of its 6 input ports are recieving logic "on".

use std::{cell::RefCell, rc::Rc};

use bevy::{
    app::{App, Update},
    prelude::{EventReader, IntoSystemConfigs, OnEnter, Query, Res, ResMut, States},
};

use crate::{
    block::Block,
    events::block_events::BlockDataSystemParams,
    logic::{logic_driver::LogicDriver, BlockLogicData, LogicBlock, LogicConnection, LogicInputEvent, LogicSystemSet, PortType},
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};

fn register_logic_ports(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(logic_indicator) = blocks.from_id("cosmos:logic_indicator") {
        registry.register(LogicBlock::new(logic_indicator, [Some(LogicConnection::Port(PortType::Input)); 6]));
    }
}

fn logic_indicator_input_event_listener(
    mut evr_logic_input: EventReader<LogicInputEvent>,
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
        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:logic_indicator" {
            continue;
        }
        let Ok(logic_driver) = q_logic_driver.get_mut(ev.entity) else {
            continue;
        };
        let Some(mut logic_data) = structure.query_block_data_mut(ev.block.coords(), &mut q_logic_data, bs_params.clone()) else {
            continue;
        };

        let new_state = BlockLogicData(
            logic_driver
                .read_all_inputs(ev.block.coords(), structure.block_rotation(ev.block.coords()))
                .iter()
                .any(|signal| *signal != 0) as i32,
        );

        if **logic_data != new_state {
            // Don't trigger unneccesary change detection.
            **logic_data = new_state;
        }
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_ports).add_systems(
        Update,
        logic_indicator_input_event_listener
            .in_set(LogicSystemSet::Consume)
            .ambiguous_with(LogicSystemSet::Consume),
    );
}
