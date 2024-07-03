use bevy::{
    app::{App, Update},
    prelude::{EventReader, EventWriter, IntoSystemConfigs, Query, Res},
};

use crate::{
    block::Block,
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
    wires::{
        wire_graph::{LogicBlock, WireGraph},
        LogicInputEvent, LogicOutputEvent, LogicSystemSet,
    },
};

fn logic_on_logic_output_event_listener(
    mut evr_logic_output: EventReader<LogicOutputEvent>,
    mut evw_logic_input: EventWriter<LogicInputEvent>,
    logic_blocks: Res<Registry<LogicBlock>>,
    blocks: Res<Registry<Block>>,
    mut q_wire_graph: Query<&mut WireGraph>,
    mut q_structure: Query<&mut Structure>,
) {
    for ev in evr_logic_output.read() {
        let Ok(structure) = q_structure.get_mut(ev.entity) else {
            continue;
        };
        if structure.block_at(ev.output_port.coords, &blocks).unlocalized_name() != "cosmos:logic_on" {
            continue;
        }
        let Ok(mut wire_graph) = q_wire_graph.get_mut(ev.entity) else {
            continue;
        };

        let logic_group = wire_graph
            .groups
            .get_mut(&ev.group_id)
            .expect("Logic On block port should have a logic group.");

        // Set this port to 'on' in it's logic group.
        logic_group.producers.insert(ev.output_port, true);

        // Notify the input ports in this port's group.
        for &input_port in logic_group.consumers.iter() {
            evw_logic_input.send(LogicInputEvent {
                group_id: ev.group_id,
                input_port,
                entity: ev.entity,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, logic_on_logic_output_event_listener.in_set(LogicSystemSet::Producing));
}
