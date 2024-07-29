//! Logic behavior for "Logic On", a block that outputs a logic signal on all 6 faces.

use bevy::{
    app::{App, Update},
    prelude::{EventReader, EventWriter, IntoSystemConfigs, OnEnter, Query, Res, ResMut, States},
};

use crate::{
    block::Block,
    logic::{
        logic_driver::LogicDriver, LogicBlock, LogicConnection, LogicOutputEvent, LogicSystemSet, Port, PortType, QueueLogicInputEvent,
    },
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};

fn register_logic_connections(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(logic_on) = blocks.from_id("cosmos:logic_on") {
        registry.register(LogicBlock::new(logic_on, [Some(LogicConnection::Port(PortType::Output)); 6]));
    }
}

fn logic_on_output_event_listener(
    mut evr_logic_output: EventReader<LogicOutputEvent>,
    mut evw_queue_logic_input: EventWriter<QueueLogicInputEvent>,
    logic_blocks: Res<Registry<LogicBlock>>,
    blocks: Res<Registry<Block>>,
    mut q_logic_driver: Query<&mut LogicDriver>,
    mut q_structure: Query<&mut Structure>,
) {
    // Internal logic signal should later be set to 1 (or some other value) with a GUI.
    for ev in evr_logic_output.read() {
        let Ok(structure) = q_structure.get_mut(ev.entity) else {
            continue;
        };
        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:logic_on" {
            continue;
        }
        let Ok(mut logic_driver) = q_logic_driver.get_mut(ev.entity) else {
            continue;
        };

        // Could cause performance problems if many of the same logic block are updated in a single frame. Might move this lookup somewhere else.
        let Some(logic_block) = logic_blocks.from_id("cosmos:logic_on") else {
            continue;
        };

        for face in logic_block.output_faces() {
            let port = Port::new(ev.block.coords(), structure.block_rotation(ev.block.coords()).direction_of(face));
            logic_driver.update_producer(port, 1, &mut evw_queue_logic_input, ev.entity);
        }
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_connections)
        .add_systems(
            Update,
            logic_on_output_event_listener
                .in_set(LogicSystemSet::Produce)
                .ambiguous_with(LogicSystemSet::Produce),
        );
}
