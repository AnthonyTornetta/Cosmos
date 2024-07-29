//! Logic behavior of the logic wire block (of every color), a block with all faces connecting to logic, but no inputs, outputs, or internal formula.

use bevy::{
    app::App,
    prelude::{OnEnter, Res, ResMut, States},
};

use crate::{
    block::Block,
    logic::{LogicBlock, LogicConnection, LogicWireColor, WireType},
    registry::{identifiable::Identifiable, Registry},
};

fn register_logic_connections(
    blocks: Res<Registry<Block>>,
    mut logic_blocks: ResMut<Registry<LogicBlock>>,
    logic_wire_colors: Res<Registry<LogicWireColor>>,
) {
    for wire_color in logic_wire_colors.iter() {
        if let Some(logic_wire) = blocks.from_id(wire_color.unlocalized_name()) {
            logic_blocks.register(LogicBlock::new(
                logic_wire,
                [Some(LogicConnection::Wire(WireType::Color(wire_color.id()))); 6],
            ));
        }
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_connections);
}
