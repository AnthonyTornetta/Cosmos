//! Contains all the logic for the client-side of Cosmos.

#![warn(missing_docs)]

pub mod asset;
pub mod audio;
pub mod block;
pub mod camera;
pub mod ecs;
pub mod entities;
pub mod events;
pub mod input;
pub mod interactions;
pub mod inventory;
pub mod lang;
pub mod loading;
pub mod materials;
pub mod music;
pub mod netty;
pub mod physics;
pub mod plugin;
pub mod projectiles;
pub mod rendering;
pub mod settings;
pub mod skybox;
pub mod state;
pub mod structure;
pub mod ui;
pub mod universe;
pub mod window;

use std::env;

use bevy_renet::transport::NetcodeClientPlugin;
use cosmos_core::netty::get_local_ipaddress;
use netty::connect::{self, HostConfig};
use netty::flags::LocalPlayer;
use netty::mapping::NetworkMapping;
use state::game_state::GameState;

use bevy::prelude::*;
use bevy_rapier3d::prelude::{RapierConfiguration, TimestepMode};
use bevy_renet::RenetClientPlugin;
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;

fn main() {
    // #[cfg(debug_assertions)]
    // env::set_var("RUST_BACKTRACE", "1");

    let args: Vec<String> = env::args().collect();

    let host_name = if args.len() > 1 {
        args.get(1).unwrap().to_owned()
    } else {
        get_local_ipaddress()
    };

    println!("Host: {host_name}");

    let mut app = App::new();

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
        .add_state::<GameState>()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(CosmosCorePluginGroup::new(
            GameState::PreLoading,
            GameState::Loading,
            GameState::PostLoading,
            GameState::Connecting,
            GameState::Playing,
        ))
        .add_plugins(RenetClientPlugin)
        .add_plugins(NetcodeClientPlugin)
        // .add_plugins(RapierDebugRenderPlugin::default())
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
    netty::register(&mut app);
    lang::register(&mut app);
    structure::register(&mut app);
    block::register(&mut app);
    projectiles::register(&mut app);
    materials::register(&mut app);
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

    app.run();
}
