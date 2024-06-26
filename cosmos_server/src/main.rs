//! Contains all the logic for the server-side of Cosmos.

#![feature(fs_try_exists)]
#![feature(get_many_mut)]
#![feature(duration_constructors)]
#![feature(iter_array_chunks)]
#![feature(iterator_try_collect)]
#![warn(missing_docs)]

use bevy::{core::TaskPoolThreadAssignmentPolicy, prelude::*};
use bevy_mod_debugdump::schedule_graph;
use bevy_rapier3d::prelude::{RapierConfiguration, TimestepMode};
use bevy_renet::{transport::NetcodeServerPlugin, RenetServerPlugin};
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;

use plugin::server_plugin::ServerPlugin;
use settings::read_server_settings;
use state::GameState;
use thread_priority::{set_current_thread_priority, ThreadPriority};

#[cfg(feature = "print-schedule")]
use bevy::log::LogPlugin;

pub mod ai;
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
pub mod registry;
pub mod rng;
pub mod settings;
pub mod shop;
pub mod state;
pub mod structure;
pub mod universe;

mod utility_runs;

fn main() {
    if set_current_thread_priority(ThreadPriority::Max).is_err() {
        warn!("Failed to set main thread priority to max - this can lead to lag.");
    } else {
        info!("Successfully set main thread priority to max!");
    }

    let server_settings = read_server_settings();

    let ip = server_settings.ip.clone();

    let mut app = App::new();

    let default_plugins = DefaultPlugins
        .set(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions {
                compute: TaskPoolThreadAssignmentPolicy {
                    min_threads: 1,
                    max_threads: usize::MAX,
                    percent: 0.25,
                },
                ..Default::default()
            },
        })
        .set(ImagePlugin::default_nearest());

    #[cfg(feature = "print-schedule")]
    let default_plugins = default_plugins.disable::<LogPlugin>();

    app
        // This must be the first thing added or systems don't get added correctly
        .init_state::<GameState>()
        .insert_resource(RapierConfiguration {
            gravity: Vec3::ZERO,
            timestep_mode: TimestepMode::Interpolated {
                dt: 1.0 / 60.0,
                time_scale: 1.0,
                substeps: 2,
            },
            ..default()
        })
        .add_plugins(default_plugins)
        .add_plugins(CosmosCorePluginGroup::new(
            GameState::PreLoading,
            GameState::Loading,
            GameState::PostLoading,
            GameState::Playing,
            GameState::Playing,
        ))
        .add_plugins((RenetServerPlugin, NetcodeServerPlugin, ServerPlugin { ip }))
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
