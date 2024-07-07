//! Logic behavior for "Logic Indicator", a block that changes its appearance to indicate if any of its 6 input ports are recieving logic "on".

use bevy::{
    app::{App, Update},
    prelude::{EventReader, EventWriter, IntoSystemConfigs, OnEnter, Query, Res, ResMut, States},
};

use crate::{
    block::Block,
    events::block_events::BlockDataChangedEvent,
    logic::{logic::Logic, LogicBlock, LogicConnection, LogicInputEvent, LogicSystemSet, PortType, LOGIC_BIT},
    registry::{identifiable::Identifiable, Registry},
    structure::{chunk::BlockInfo, Structure},
};

fn register_logic_ports(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(light) = blocks.from_id("cosmos:logic_indicator") {
        registry.register(LogicBlock::new(light, [Some(LogicConnection::Port(PortType::Input)); 6]));
    }
}

/// Wraps the Boolean state of an indicator block being on or off.
pub trait LogicIndicatorBlockInfo {
    /// Returns [`true`] if the indicator is on, [`false`] otherwise.
    fn indicator_on(&self) -> bool;
    /// Turns the light on.
    fn set_indicator_on(&mut self);
    /// Turns the light off.
    fn set_indicator_off(&mut self);
}

impl LogicIndicatorBlockInfo for BlockInfo {
    fn indicator_on(&self) -> bool {
        (self.0 >> LOGIC_BIT) & 1 == 0
    }

    // 'On' has the 8th bit (from right to left) set to zero.
    fn set_indicator_on(&mut self) {
        self.0 &= !(1 << LOGIC_BIT);
    }

    fn set_indicator_off(&mut self) {
        self.0 |= 1 << LOGIC_BIT;
    }
}

fn logic_indicator_logic_input_event_listener(
    mut evw_block_data_changed: EventWriter<BlockDataChangedEvent>,
    mut evr_logic_input: EventReader<LogicInputEvent>,
    blocks: Res<Registry<Block>>,
    mut q_logic: Query<&mut Logic>,
    mut q_structure: Query<&mut Structure>,
) {
    for ev in evr_logic_input.read() {
        let Ok(mut structure) = q_structure.get_mut(ev.entity) else {
            continue;
        };
        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:logic_indicator" {
            continue;
        }
        let Ok(logic) = q_logic.get_mut(ev.entity) else {
            continue;
        };

        let on = logic
            .global_port_inputs(ev.block.coords(), structure.block_rotation(ev.block.coords()))
            .iter()
            .any(|port_on| *port_on);

        let mut block_info = structure.block_info_at(ev.block.coords());
        if on {
            block_info.set_indicator_on();
        } else {
            block_info.set_indicator_off();
        }
        structure.set_block_info_at(ev.block.coords(), block_info, &mut evw_block_data_changed);
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_ports)
        .add_systems(Update, logic_indicator_logic_input_event_listener.in_set(LogicSystemSet::Consuming));
}
