//! Logic behavior for "Logic On", a block that outputs a logic signal on all 6 faces.

use bevy::{
    app::{App, Update},
    prelude::{EventReader, EventWriter, IntoSystemConfigs, OnEnter, Query, Res, ResMut, States},
};

use crate::{
    block::Block,
    logic::{
        default_logic_block_output, logic_driver::LogicDriver, BlockLogicData, LogicBlock, LogicConnection, LogicOutputEvent,
        LogicSystemSet, PortType, QueueLogicInputEvent,
    },
    registry::Registry,
    structure::Structure,
};

fn register_logic_connections(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(logic_on) = blocks.from_id("cosmos:logic_on") {
        registry.register(LogicBlock::new(logic_on, [Some(LogicConnection::Port(PortType::Output)); 6]));
    }
}

fn logic_on_output_event_listener(
    evr_logic_output: EventReader<LogicOutputEvent>,
    evw_queue_logic_input: EventWriter<QueueLogicInputEvent>,
    logic_blocks: Res<Registry<LogicBlock>>,
    blocks: Res<Registry<Block>>,
    q_logic_driver: Query<&mut LogicDriver>,
    q_structure: Query<&mut Structure>,
    q_logic_data: Query<&BlockLogicData>,
) {
    default_logic_block_output(
        "cosmos:logic_on",
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
        .add_systems(Update, logic_on_output_event_listener.in_set(LogicSystemSet::Produce));
}
