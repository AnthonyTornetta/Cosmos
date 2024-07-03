use bevy::{
    app::{App, Update},
    prelude::{EventReader, Query, Res},
};

use crate::{
    block::Block,
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
    wires::{
        wire_graph::{LogicBlock, WireGraph},
        LogicInputEvent,
    },
};

fn light_logic_input_event_listener(
    mut evr_logic_input: EventReader<LogicInputEvent>,
    logic_blocks: Res<Registry<LogicBlock>>,
    blocks: Res<Registry<Block>>,
    mut q_wire_graph: Query<&mut WireGraph>,
    mut q_structure: Query<&mut Structure>,
) {
    for ev in evr_logic_input.read() {
        let Ok(structure) = q_structure.get_mut(ev.entity) else {
            continue;
        };
        if structure.block_at(ev.input_port.coords, &blocks).unlocalized_name() != "cosmos:light" {
            continue;
        }
        let Ok(mut wire_graph) = q_wire_graph.get_mut(ev.entity) else {
            continue;
        };

        let logic_group = wire_graph
            .groups
            .get_mut(&ev.logic_group_id)
            .expect("Light block port should have a logic group.");

        if logic_group.on() {
            // Turn light off.
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, light_logic_input_event_listener);
}
