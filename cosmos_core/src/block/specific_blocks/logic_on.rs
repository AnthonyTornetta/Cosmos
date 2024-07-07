//! Logic behavior for "Logic On", a block that outputs a logic signal on all 6 faces.

use bevy::{
    app::{App, Update},
    prelude::{EventReader, EventWriter, IntoSystemConfigs, OnEnter, Query, Res, ResMut, States},
};

use crate::{
    block::Block,
    logic::{logic_driver::LogicDriver, LogicBlock, LogicConnection, LogicInputEvent, LogicOutputEvent, LogicSystemSet, Port, PortType},
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};

fn register_logic_connections(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(logic_on) = blocks.from_id("cosmos:logic_on") {
        registry.register(LogicBlock::new(logic_on, [Some(LogicConnection::Port(PortType::Output)); 6]));
    }
}

fn logic_on_logic_output_event_listener(
    mut evr_logic_output: EventReader<LogicOutputEvent>,
    mut evw_logic_input: EventWriter<LogicInputEvent>,
    blocks: Res<Registry<Block>>,
    mut q_logic_driver: Query<&mut LogicDriver>,
    mut q_structure: Query<&mut Structure>,
) {
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

        // Set all this block's ports to "on" in their logic groups.
        for local_face in structure.block_rotation(ev.block.coords()).all_local_faces() {
            let port = Port::new(ev.block.coords(), local_face);
            logic_driver.update_producer(port, true, &mut evw_logic_input, ev.entity);
        }
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_connections)
        .add_systems(Update, logic_on_logic_output_event_listener.in_set(LogicSystemSet::Producing));
}
