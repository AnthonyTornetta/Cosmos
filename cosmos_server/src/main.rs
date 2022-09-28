use bevy::prelude::*;
use bevy::winit::WinitPlugin;
use bevy_inspector_egui::WorldInspectorPlugin;
use bevy_renet::RenetServerPlugin;
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;

use plugin::server_plugin::ServerPlugin;
use state::GameState;

pub mod blocks;
pub mod events;
pub mod init;
pub mod netty;
pub mod physics;
pub mod plugin;
pub mod server;
pub mod state;
pub mod structure;

fn main() {
    App::new()
        .add_plugins(CosmosCorePluginGroup::new(
            GameState::PreLoading,
            GameState::Loading,
            GameState::PostLoading,
            GameState::Playing,
            GameState::Playing,
        ))
        .add_plugin(RenetServerPlugin)
        .add_plugin(WinitPlugin::default())
        .add_plugin(ServerPlugin)
        .add_plugin(WorldInspectorPlugin::new())
        .run();
}
