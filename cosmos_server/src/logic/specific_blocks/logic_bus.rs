//! Logic behavior of the logic bus block, a block with all faces connecting to logic, but no inputs, outputs, or internal formula.

use bevy::{
    app::App,
    prelude::{OnEnter, Res, ResMut, States},
};

use cosmos_core::{block::Block, registry::Registry};

use crate::logic::{LogicBlock, LogicConnection, WireType};

fn register_logic_connections(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(logic_bus) = blocks.from_id("cosmos:logic_bus") {
        registry.register(LogicBlock::new(logic_bus, [Some(LogicConnection::Wire(WireType::Bus)); 6]));
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_connections);
}
