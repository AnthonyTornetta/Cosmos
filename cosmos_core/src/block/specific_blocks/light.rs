use bevy::{
    app::{App, Update},
    prelude::{EventReader, Query, Res},
};

use crate::{
    registry::Registry,
    structure::Structure,
    wires::{
        wire_graph::{LogicBlock, WireGraph},
        LogicInputEvent,
    },
};

fn light_logic_input_event_listener(
    mut evr_logic_input: EventReader<LogicInputEvent>,
    logic_blocks: Res<Registry<LogicBlock>>,
    mut q_wire_graph: Query<&mut WireGraph>,
    mut q_structure: Query<&mut Structure>,
) {
    // for ev in evr_logic_input.read() {
    //     let Ok(structure) = q_structure.get_mut(ev.entity) else {
    //         return;
    //     };
    //     let Ok(mut wire_graph) = q_wire_graph.get_mut(ev.entity) else {
    //         return;
    //     };
    // }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, light_logic_input_event_listener);
}
