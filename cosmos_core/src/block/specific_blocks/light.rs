use bevy::{
    app::{App, Update},
    prelude::{EventReader, EventWriter, IntoSystemConfigs, Query, Res},
};

use crate::{
    block::{Block, BlockFace, ALL_BLOCK_FACES},
    events::block_events::BlockDataChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{chunk::BlockInfo, Structure},
    wires::{
        wire_graph::{LogicBlock, Port, WireGraph},
        LogicInputEvent, LogicSystemSet,
    },
};

pub trait LightBlockInfo {
    fn light_on(&self) -> bool;
    fn set_light_on(&mut self);
    fn set_light_off(&mut self);
}

impl LightBlockInfo for BlockInfo {
    fn light_on(&self) -> bool {
        (self.0 >> 7) & 1 == 0
    }

    // 'On' has the 8th bit (from right to left) set to zero.
    fn set_light_on(&mut self) {
        self.0 &= !(1 << 7);
    }

    fn set_light_off(&mut self) {
        self.0 |= 1 << 7;
    }
}

fn light_logic_input_event_listener(
    mut evw_block_data_changed: EventWriter<BlockDataChangedEvent>,
    mut evr_logic_input: EventReader<LogicInputEvent>,
    blocks: Res<Registry<Block>>,
    mut q_wire_graph: Query<&mut WireGraph>,
    mut q_structure: Query<&mut Structure>,
) {
    for ev in evr_logic_input.read() {
        let Ok(mut structure) = q_structure.get_mut(ev.entity) else {
            continue;
        };
        if structure.block_at(ev.input_port.coords, &blocks).unlocalized_name() != "cosmos:light" {
            continue;
        }
        let Ok(wire_graph) = q_wire_graph.get_mut(ev.entity) else {
            continue;
        };

        // Do I need block face in the event? Or just coords?
        // Make this a super cool method in WireGraph (which uses the global faces).
        let mut logic_on = false;
        for face in ALL_BLOCK_FACES {
            let group_id = wire_graph
                .group_of_input_port
                .get(&Port::new(ev.input_port.coords, face))
                .expect("Light input group ID should exist.");
            logic_on = logic_on || wire_graph.groups.get(group_id).expect("Light input group should exist.").on();
        }

        // Logic on means light off.
        let mut block_info = structure.block_info_at(ev.input_port.coords);
        if logic_on {
            block_info.set_light_off();
        } else {
            block_info.set_light_on();
        }
        structure.set_block_info_at(ev.input_port.coords, block_info, &mut evw_block_data_changed);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, light_logic_input_event_listener.in_set(LogicSystemSet::Consuming));
}
