use bevy::prelude::*;
use bevy::winit::WinitPlugin;
use bevy_rapier3d::prelude::RapierConfiguration;
use bevy_renet::RenetServerPlugin;
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;

use plugin::server_plugin::ServerPlugin;
use state::GameState;

pub mod blocks;
pub mod commands;
pub mod events;
pub mod init;
pub mod inventory;
pub mod netty;
pub mod physics;
pub mod plugin;
pub mod projectiles;
pub mod server;
pub mod state;
pub mod structure;

fn main() {
    App::new()
        .insert_resource(RapierConfiguration {
            gravity: Vec3::ZERO,
            ..default()
        })
        .add_plugins(CosmosCorePluginGroup::new(
            GameState::PreLoading,
            GameState::Loading,
            GameState::PostLoading,
            GameState::Playing,
            GameState::Playing,
        ))
        .add_plugin(RenetServerPlugin::default())
        .add_plugin(WinitPlugin::default())
        .add_plugin(ServerPlugin)
        .run();
}
