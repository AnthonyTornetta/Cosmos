//! Contains all the logic for the server-side of Cosmos.

#![feature(duration_constructors)]
#![feature(iter_array_chunks)]
#![feature(iterator_try_collect)]
#![feature(duration_constructors_lite)]
#![warn(missing_docs)]
// This one has a stupid rule where if you have `fn (&self) -> HasLifetime`, you need to do `fn (&self) -> HasLifetime<'_>`. This is stupid.
#![allow(mismatched_lifetime_syntaxes)]

use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, SystemInformationDiagnosticsPlugin},
    prelude::*,
};
use bevy_mod_debugdump::schedule_graph;
use bevy_rapier3d::plugin::{RapierContextInitialization, RapierPhysicsPlugin};
use bevy_renet::{RenetServerPlugin, netcode::NetcodeServerPlugin};
use cosmos_core::{
    netty::sync::registry::RegistrySyncInit, physics::collision_handling::CosmosPhysicsFilter,
    plugin::cosmos_core_plugin::CosmosCorePluginGroup, state::GameState,
};

// use iyes_perf_ui::PerfUiPlugin;
use plugin::server_plugin::ServerPlugin;
use settings::read_server_settings;

#[cfg(feature = "print-schedule")]
use bevy::log::LogPlugin;

pub mod ai;
pub mod blocks;
pub mod chat;
pub mod commands;
pub mod coms;
mod converters;
pub mod crafting;
pub mod creative;
mod debug;
mod economy;
pub mod entities;
pub mod faction;
pub mod fluid;
pub mod init;
pub mod inventory;
pub mod items;
pub mod logic;
pub mod loot;
pub mod netty;
pub mod persistence;
pub mod physics;
pub mod plugin;
pub mod projectiles;
pub mod quest;
pub mod rng;
pub mod server;
pub mod settings;
pub mod shop;
pub mod structure;
pub mod universe;

mod utility_runs;

fn main() {
    let server_settings = read_server_settings();

    let port = server_settings.port.unwrap_or(1337);

    let mut app = App::new();

    let default_plugins = DefaultPlugins
        .set(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions {
                // compute: TaskPoolThreadAssignmentPolicy {
                //     min_threads: 1,
                //     max_threads: usize::MAX,
                //     percent: 0.25,
                //     ..default()
                // },
                ..Default::default()
            },
        })
        .set(ImagePlugin::default_nearest());

    #[cfg(feature = "print-schedule")]
    let default_plugins = default_plugins.disable::<LogPlugin>();

    app
        // .insert_resource(TimestepMode::Interpolated {
        //     dt: 1.0 / 60.0,
        //     time_scale: 1.0,
        //     substeps: 2,
        // })
        .add_plugins(default_plugins)
        // This must be the first thing added or systems don't get added correctly, but after default plugins.
        .init_state::<GameState>()
        .add_plugins(CosmosCorePluginGroup::new(
            GameState::PreLoading,
            GameState::Loading,
            GameState::PostLoading,
            GameState::Playing,
            GameState::Playing,
            RegistrySyncInit::Server {
                playing_state: GameState::Playing,
            },
        ))
        .add_plugins(
            RapierPhysicsPlugin::<CosmosPhysicsFilter>::default()
                // .in_schedule(FixedUpdate)
                .with_custom_initialization(RapierContextInitialization::NoAutomaticRapierContext),
        )
        .add_plugins((
            RenetServerPlugin,
            NetcodeServerPlugin,
            ServerPlugin { port },
            // Used for diagnostics
            SystemInformationDiagnosticsPlugin,
            EntityCountDiagnosticsPlugin,
            FrameTimeDiagnosticsPlugin::default(),
            // PerfUiPlugin,
        ))
        .insert_resource(server_settings);

    if cfg!(feature = "print-schedule") {
        println!(
            "{}",
            bevy_mod_debugdump::schedule_graph_dot(
                &mut app,
                Update,
                &schedule_graph::Settings {
                    ambiguity_enable: false,
                    ambiguity_enable_on_world: false,
                    ..Default::default()
                }
            )
        );
        return;
    }

    app.run();
}
