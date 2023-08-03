//! Contains all the logic for the server-side of Cosmos.

#![feature(fs_try_exists)]
#![warn(missing_docs)]

use std::env;

use bevy::prelude::*;
use bevy_rapier3d::prelude::{RapierConfiguration, TimestepMode};
use bevy_renet::{transport::NetcodeServerPlugin, RenetServerPlugin};
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;

use plugin::server_plugin::ServerPlugin;
use state::GameState;

pub mod blocks;
pub mod commands;
pub mod entities;
pub mod events;
pub mod init;
pub mod inventory;
pub mod netty;
pub mod persistence;
pub mod physics;
pub mod plugin;
pub mod projectiles;
pub mod rng;
pub mod state;
pub mod structure;
pub mod universe;

fn main() {
    // #[cfg(debug_assertions)]
    // env::set_var("RUST_BACKTRACE", "1");

    let args: Vec<String> = env::args().collect();

    let ip = if args.len() > 1 {
        Some(args.get(1).unwrap().to_owned())
    } else {
        None
    };

    App::new()
        // This must be the first thing added or systems don't get added correctly
        .add_state::<GameState>()
        .insert_resource(RapierConfiguration {
            gravity: Vec3::ZERO,
            timestep_mode: TimestepMode::Interpolated {
                dt: 1.0 / 60.0,
                time_scale: 1.0,
                substeps: 2,
            },
            ..default()
        })
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(CosmosCorePluginGroup::new(
            GameState::PreLoading,
            GameState::Loading,
            GameState::PostLoading,
            GameState::Playing,
            GameState::Playing,
        ))
        .add_plugins((RenetServerPlugin, NetcodeServerPlugin, ServerPlugin { ip }))
        .run();
}
