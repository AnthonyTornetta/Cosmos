//! Logic behavior for the button, a block that outputs a logic signal on all 6 faces when on for a
//! short period, before turning off again.

use bevy::prelude::*;

use cosmos_core::{
    block::{
        Block,
        block_events::{BlockEventsSet, BlockInteractEvent},
        data::BlockData,
    },
    events::block_events::BlockDataSystemParams,
    netty::sync::IdentifiableComponent,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{Structure, chunk::BlockInfo},
};
use serde::{Deserialize, Serialize};

use crate::{
    logic::{
        LOGIC_BIT, LogicBlock, LogicConnection, LogicOutputEvent, LogicSystemSet, Port, PortType, QueueLogicInputEvent,
        logic_driver::LogicDriver,
    },
    persistence::make_persistent::{DefaultPersistentComponent, make_persistent},
};

const BLOCK_ID: &str = "cosmos:button";

#[derive(Component, Debug, Serialize, Deserialize)]
struct ButtonTimer(u8);

impl IdentifiableComponent for ButtonTimer {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:button_timer"
    }
}

impl DefaultPersistentComponent for ButtonTimer {}

const MAX_BUTTON_TICKS: u8 = 10;

fn register_logic_connections(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(logic_on) = blocks.from_id(BLOCK_ID) {
        registry.register(LogicBlock::new(logic_on, [Some(LogicConnection::Port(PortType::Output)); 6]));
    }
}

fn on_interact_with_button(
    mut evr_interact: EventReader<BlockInteractEvent>,
    mut q_structure: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut bs_params: BlockDataSystemParams,
    mut q_block_data: Query<&mut BlockData>,
    q_has_data: Query<(), With<ButtonTimer>>,
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
        data.0 |= LOGIC_BIT;

        structure.set_block_info_at(block.coords(), data, &mut bs_params.ev_writer);
        structure.insert_block_data(block.coords(), ButtonTimer(0), &mut bs_params, &mut q_block_data, &q_has_data);
    }
}

fn tick_button_down(
    mut q_block_data: Query<&mut BlockData>,
    mut q_block_timer: Query<(Entity, &mut ButtonTimer)>,
    mut q_structure: Query<&mut Structure>,
    mut bs_params: BlockDataSystemParams,
    q_data: Query<(), With<ButtonTimer>>,
) {
    for (ent, mut timer) in q_block_timer.iter_mut() {
        let Ok(bd) = q_block_data.get(ent) else {
            continue;
        };

        timer.0 += 1;

        if timer.0 < MAX_BUTTON_TICKS {
            continue;
        }

        let Ok(mut structure) = q_structure.get_mut(bd.identifier.block.structure()) else {
            continue;
        };

        let info = structure.block_info_at(bd.identifier.block.coords());
        structure.set_block_info_at(
            bd.identifier.block.coords(),
            BlockInfo(info.0 & !LOGIC_BIT),
            &mut bs_params.ev_writer,
        );
        structure.remove_block_data(bd.identifier.block.coords(), &mut bs_params, &mut q_block_data, &q_data);
    }
}

fn logic_on_output_event_listener(
    mut evr_logic_output: EventReader<LogicOutputEvent>,
    mut evw_queue_logic_input: MessageWriter<QueueLogicInputEvent>,
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
    make_persistent::<ButtonTimer>(app);

    app.add_systems(OnEnter(GameState::PostLoading), register_logic_connections)
        .add_systems(
            FixedUpdate,
            (
                tick_button_down.in_set(LogicSystemSet::PreLogicTick),
                logic_on_output_event_listener
                    .in_set(LogicSystemSet::Produce)
                    .ambiguous_with(LogicSystemSet::Produce),
            ),
        )
        .add_systems(FixedUpdate, on_interact_with_button.in_set(BlockEventsSet::ProcessEvents));
}
