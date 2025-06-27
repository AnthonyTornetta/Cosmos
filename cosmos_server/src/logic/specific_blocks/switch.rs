//! Logic behavior for the switch, a block that outputs a logic signal on all 6 faces when on.

use bevy::prelude::*;

use cosmos_core::{
    block::{
        Block,
        block_events::{BlockEventsSet, BlockInteractEvent},
    },
    events::block_events::BlockDataChangedEvent,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::Structure,
};

use crate::logic::{
    LOGIC_BIT, LogicBlock, LogicConnection, LogicOutputEvent, LogicSystemSet, Port, PortType, QueueLogicInputEvent,
    logic_driver::LogicDriver,
};

const BLOCK_ID: &str = "cosmos:switch";

fn register_logic_connections(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(logic_on) = blocks.from_id(BLOCK_ID) {
        registry.register(LogicBlock::new(logic_on, [Some(LogicConnection::Port(PortType::Output)); 6]));
    }
}

fn on_interact_with_switch(
    mut evr_interact: EventReader<BlockInteractEvent>,
    mut q_structure: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut evw_block_data_changed: EventWriter<BlockDataChangedEvent>,
) {
    for ev in evr_interact.read() {
        let Some(block) = ev.block else {
            continue;
        };

        let Ok(mut structure) = q_structure.get_mut(block.structure()) else {
            continue;
        };
        if structure.block_at(block.coords(), &blocks).unlocalized_name() != BLOCK_ID {
            continue;
        }

        let mut data = structure.block_info_at(block.coords());
        data.0 ^= LOGIC_BIT;
        structure.set_block_info_at(block.coords(), data, &mut evw_block_data_changed);
    }
}

fn logic_on_output_event_listener(
    mut evr_logic_output: EventReader<LogicOutputEvent>,
    mut evw_queue_logic_input: EventWriter<QueueLogicInputEvent>,
    logic_blocks: Res<Registry<LogicBlock>>,
    blocks: Res<Registry<Block>>,
    mut q_logic_driver: Query<&mut LogicDriver>,
    q_structure: Query<&Structure>,
) {
    // Internal logic signal should later be set to 1 (or some other value) with a GUI.
    for ev in evr_logic_output.read() {
        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            continue;
        };
        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != BLOCK_ID {
            continue;
        }
        let Ok(mut logic_driver) = q_logic_driver.get_mut(ev.block.structure()) else {
            continue;
        };
        let Some(logic_block) = logic_blocks.from_id(BLOCK_ID) else {
            continue;
        };

        let signal = ((structure.block_info_at(ev.block.coords()).0 & LOGIC_BIT) as i32).signum();

        for face in logic_block.output_faces() {
            let port = Port::new(ev.block.coords(), structure.block_rotation(ev.block.coords()).direction_of(face));
            logic_driver.update_producer(port, signal, &mut evw_queue_logic_input, ev.block.structure());
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), register_logic_connections)
        .add_systems(
            FixedUpdate,
            logic_on_output_event_listener
                .in_set(LogicSystemSet::Produce)
                .ambiguous_with(LogicSystemSet::Produce),
        )
        .add_systems(FixedUpdate, on_interact_with_switch.in_set(BlockEventsSet::ProcessEvents));
}
