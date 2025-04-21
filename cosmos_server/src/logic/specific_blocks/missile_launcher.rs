use bevy::{
    app::App,
    prelude::{OnEnter, Res, ResMut, States},
};

use cosmos_core::{block::Block, registry::Registry};

use crate::logic::{LogicBlock, LogicConnection, PortType};

fn register_logic_connections_for_missile_launcher(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(block) = blocks.from_id("cosmos:missile_launcher") {
        registry.register(LogicBlock::new(block, [Some(LogicConnection::Port(PortType::Input)); 6]));
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_connections_for_missile_launcher);
}
