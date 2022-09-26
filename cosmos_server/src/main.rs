pub mod blocks;
pub mod events;
pub mod init;
pub mod netty;
pub mod physics;
pub mod plugin;
pub mod server;
pub mod structure;

use bevy::prelude::*;
use bevy::winit::WinitPlugin;
use bevy_renet::RenetServerPlugin;
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;
use plugin::server_plugin::ServerPlugin;

fn main() {
    App::new()
        .add_plugins(CosmosCorePluginGroup::default())
        .add_plugin(RenetServerPlugin)
        .add_plugin(WinitPlugin::default())
        .add_plugin(ServerPlugin)
        .run();
}
