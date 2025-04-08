//! Contains all the logic for the client-side of Cosmos.

#![warn(missing_docs)]
#![feature(iter_array_chunks)]
#![feature(array_windows)]

pub mod asset;
pub mod audio;
pub mod block;
pub mod camera;
pub mod chat;
pub mod coms;
pub mod crafting;
pub mod debug;
pub mod economy;
pub mod ecs;
pub mod entities;
pub mod events;
pub mod input;
pub mod interactions;
pub mod inventory;
pub mod item;
pub mod lang;
pub mod loading;
pub mod netty;
pub mod physics;
pub mod plugin;
pub mod projectiles;
pub mod quest;
pub mod rendering;
pub mod settings;
pub mod shop;
pub mod skybox;
pub mod structure;
pub mod ui;
pub mod universe;
pub mod window;

use bevy::diagnostic::{EntityCountDiagnosticsPlugin, SystemInformationDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::WindowMode;
use bevy::{core::TaskPoolThreadAssignmentPolicy, diagnostic::FrameTimeDiagnosticsPlugin};
use bevy_hanabi::HanabiPlugin;
// use bevy_mod_billboard::plugin::BillboardPlugin;
use bevy_mod_debugdump::schedule_graph;
use bevy_obj::ObjPlugin;

use bevy_rapier3d::plugin::{RapierContextInitialization, RapierPhysicsPlugin};
// use bevy_rapier3d::render::RapierDebugRenderPlugin;
use bevy_renet2::{transport::NetcodeClientPlugin, RenetClientPlugin};
use clap::{arg, Parser};
use cosmos_core::netty::sync::registry::RegistrySyncInit;
use cosmos_core::state::GameState;
use cosmos_core::{physics::collision_handling::CosmosPhysicsFilter, plugin::cosmos_core_plugin::CosmosCorePluginGroup};
// use iyes_perf_ui::PerfUiPlugin;
use netty::connect::{self};
use thread_priority::{set_current_thread_priority, ThreadPriority};

#[cfg(feature = "print-schedule")]
use bevy::log::LogPlugin;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Connection string of the server to connect to (ip/url:port)
    #[arg(long)]
    server: Option<String>,

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

    // let host_name = args.ip.unwrap_or_else(get_local_ipaddress);

    // info!("Host: {host_name}");

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
                    WindowMode::BorderlessFullscreen(MonitorSelection::Current)
                } else {
                    WindowMode::Windowed
                },
                // for panorama generation:
                // resolution: WindowResolution::new(1000.0, 1000.0),
                // decorations: false,
                ..Default::default()
            }),
            ..Default::default()
        })
        .set(ImagePlugin::default_nearest());

    #[cfg(feature = "print-schedule")]
    let default_plugins = default_plugins.disable::<LogPlugin>();

    app
        // .insert_resource(HostConfig { host_name })
        // .insert_resource(TimestepMode::Interpolated {
        //     dt: 1.0 / 60.0,
        //     time_scale: 1.0,
        //     substeps: 2,
        // })
        .insert_resource(ClearColor(Color::BLACK))
        // This must be registered here, before it is used anywhere
        .add_plugins(default_plugins)
        .init_state::<GameState>()
        .add_plugins(CosmosCorePluginGroup::new(
            GameState::PreLoading,
            GameState::Loading,
            GameState::PostLoading,
            GameState::MainMenu,
            GameState::Playing,
            RegistrySyncInit::Client {
                connecting_state: GameState::Connecting,
                loading_data_state: GameState::LoadingData,
                loading_world_state: GameState::LoadingWorld,
            },
        ))
        .add_plugins(
            RapierPhysicsPlugin::<CosmosPhysicsFilter>::default()
                // .in_schedule(FixedUpdate)
                .with_custom_initialization(RapierContextInitialization::default()),
        )
        .add_plugins((
            RenetClientPlugin,
            NetcodeClientPlugin,
            ObjPlugin,
            HanabiPlugin,
            // Used for diagnostics
            SystemInformationDiagnosticsPlugin,
            EntityCountDiagnosticsPlugin,
            FrameTimeDiagnosticsPlugin,
            // PerfUiPlugin,
            // BillboardPlugin,
        ))
        // If you enable rapier debug, make sure to disable order independent transparency
        // on camera.
        // .add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(OnEnter(GameState::Connecting), connect::establish_connection)
        .add_systems(Update, connect::wait_for_connection.run_if(in_state(GameState::Connecting)));

    input::register(&mut app);
    window::register(&mut app);
    asset::register(&mut app);
    audio::register(&mut app);
    events::register(&mut app);
    interactions::register(&mut app);
    camera::register(&mut app);
    ui::register(&mut app);
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
    settings::register(&mut app);
    physics::register(&mut app);
    ecs::register(&mut app);
    shop::register(&mut app);
    economy::register(&mut app);
    item::register(&mut app);
    debug::register(&mut app);
    chat::register(&mut app);
    crafting::register(&mut app);
    coms::register(&mut app);
    quest::register(&mut app);

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
