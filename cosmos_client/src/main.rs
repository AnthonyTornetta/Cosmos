//! Contains all the logic for the client-side of Cosmos.

#![warn(missing_docs)]
#![feature(iter_array_chunks)]

pub mod asset;
pub mod audio;
pub mod block;
pub mod camera;
pub mod economy;
pub mod ecs;
pub mod entities;
pub mod events;
pub mod fluid;
pub mod input;
pub mod interactions;
pub mod inventory;
pub mod item;
pub mod lang;
pub mod loading;
pub mod music;
pub mod netty;
pub mod physics;
pub mod plugin;
pub mod projectiles;
pub mod registry;
pub mod rendering;
pub mod settings;
pub mod shop;
pub mod skybox;
pub mod state;
pub mod structure;
pub mod ui;
pub mod universe;
pub mod window;

use bevy::core::TaskPoolThreadAssignmentPolicy;
use bevy::prelude::*;
use bevy::window::WindowMode;
use bevy_hanabi::HanabiPlugin;
use bevy_mod_debugdump::schedule_graph;
use bevy_obj::ObjPlugin;
use bevy_rapier3d::{
    prelude::{RapierConfiguration, TimestepMode},
    render::RapierDebugRenderPlugin,
};
use bevy_renet::transport::NetcodeClientPlugin;
use bevy_renet::RenetClientPlugin;
use clap::{arg, Parser};
use cosmos_core::netty::get_local_ipaddress;
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;
use netty::connect::{self, HostConfig};
use state::game_state::GameState;
use thread_priority::{set_current_thread_priority, ThreadPriority};

#[cfg(feature = "print-schedule")]
use bevy::log::LogPlugin;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Ip of the server to connect to
    #[arg(long)]
    ip: Option<String>,

    /// If this is fullscreen, the app will start in fullscreen
    #[arg(short, long, default_value_t = false)]
    fullscreen: bool,
}

fn main() {
    if set_current_thread_priority(ThreadPriority::Max).is_err() {
        warn!("Failed to set main thread priority to max - this can lead to lag.");
    } else {
        info!("Successfully set main thread priority to max!");
    }

    let args = Args::parse();

    let host_name = args.ip.unwrap_or_else(get_local_ipaddress);

    info!("Host: {host_name}");

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
        .set(WindowPlugin {
            primary_window: Some(Window {
                mode: if args.fullscreen {
                    WindowMode::BorderlessFullscreen
                } else {
                    WindowMode::Windowed
                },
                ..Default::default()
            }),
            ..Default::default()
        })
        .set(ImagePlugin::default_nearest());

    #[cfg(feature = "print-schedule")]
    let default_plugins = default_plugins.disable::<LogPlugin>();

    app.insert_resource(HostConfig { host_name })
        .insert_resource(RapierConfiguration {
            gravity: Vec3::ZERO,
            timestep_mode: TimestepMode::Interpolated {
                dt: 1.0 / 60.0,
                time_scale: 1.0,
                substeps: 2,
            },
            ..default()
        })
        .insert_resource(ClearColor(Color::BLACK))
        // This must be registered here, before it is used anywhere
        .init_state::<GameState>()
        .add_plugins(default_plugins)
        .add_plugins(CosmosCorePluginGroup::new(
            GameState::PreLoading,
            GameState::Loading,
            GameState::PostLoading,
            GameState::Connecting,
            GameState::Playing,
        ))
        .add_plugins((RenetClientPlugin, NetcodeClientPlugin, ObjPlugin, HanabiPlugin))
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(OnEnter(GameState::Connecting), connect::establish_connection)
        .add_systems(Update, connect::wait_for_connection.run_if(in_state(GameState::Connecting)))
        .add_systems(Update, connect::wait_for_done_loading.run_if(in_state(GameState::LoadingWorld)));

    input::register(&mut app);
    window::register(&mut app);
    asset::register(&mut app);
    audio::register(&mut app);
    events::register(&mut app);
    interactions::register(&mut app);
    camera::register(&mut app);
    ui::register(&mut app);
    registry::register(&mut app);
    netty::register(&mut app);
    lang::register(&mut app);
    structure::register(&mut app);
    block::register(&mut app);
    projectiles::register(&mut app);
    loading::register(&mut app);
    entities::register(&mut app);
    inventory::register(&mut app);
    rendering::register(&mut app);
    universe::register(&mut app);
    skybox::register(&mut app);
    music::register(&mut app);
    settings::register(&mut app);
    physics::register(&mut app);
    ecs::register(&mut app);
    shop::register(&mut app);
    economy::register(&mut app);
    fluid::register(&mut app);

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
