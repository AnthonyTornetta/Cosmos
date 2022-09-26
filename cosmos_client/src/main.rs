pub mod asset;
pub mod camera;
pub mod events;
pub mod input;
pub mod interactions;
pub mod netty;
pub mod plugin;
pub mod rendering;
pub mod state;
pub mod structure;
pub mod ui;
pub mod window;

use std::env;

use asset::asset_loading;
use camera::camera_controller;
use cosmos_core::netty::netty::get_local_ipaddress;
use input::inputs::{self, CosmosInputHandler, CosmosInputs};
use interactions::block_interactions;
use netty::connect::{self, ConnectionConfig};
use netty::flags::LocalPlayer;
use netty::gameplay::{receiver, sync};
use netty::mapping::NetworkMapping;
use rendering::structure_renderer;
use state::game_state::{self, GameState};
use structure::chunk_retreiver;
use ui::crosshair;

use crate::plugin::client_plugin::ClientPluginGroup;
use crate::rendering::structure_renderer::monitor_block_updates_system;
use crate::rendering::uv_mapper::UVMapper;
use bevy::prelude::*;
use bevy::render::texture::ImageSettings;
use bevy_rapier3d::prelude::{Vect, Velocity};
use bevy_renet::RenetClientPlugin;
use cosmos_core::physics::structure_physics::{
    listen_for_new_physics_event, listen_for_structure_event,
};
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;

fn process_player_movement(
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    time: Res<Time>,
    input_handler: ResMut<CosmosInputHandler>,
    mut query: Query<&mut Velocity, With<LocalPlayer>>,
    cam_query: Query<&Transform, With<Camera>>,
) {
    let mut velocity = query.single_mut();

    let cam_trans = cam_query.single();

    let max_speed: f32 = match input_handler.check_pressed(CosmosInputs::Sprint, &keys, &mouse) {
        false => 5.0,
        true => 20.0,
    };

    let mut forward = cam_trans.forward().clone();
    let mut right = cam_trans.right().clone();
    let up = Vect::new(0.0, 1.0, 0.0);

    forward.y = 0.0;
    right.y = 0.0;

    forward = forward.normalize_or_zero() * 100.0;
    right = right.normalize_or_zero() * 100.0;

    let time = time.delta_seconds();

    if input_handler.check_pressed(CosmosInputs::MoveForward, &keys, &mouse) {
        velocity.linvel += forward * time;
    }
    if input_handler.check_pressed(CosmosInputs::MoveBackward, &keys, &mouse) {
        velocity.linvel -= forward * time;
    }
    if input_handler.check_just_pressed(CosmosInputs::MoveUpOrJump, &keys, &mouse) {
        velocity.linvel += up * 5.0;
    }
    if input_handler.check_pressed(CosmosInputs::MoveLeft, &keys, &mouse) {
        velocity.linvel -= right * time;
    }
    if input_handler.check_pressed(CosmosInputs::MoveRight, &keys, &mouse) {
        velocity.linvel += right * time;
    }
    if input_handler.check_pressed(CosmosInputs::SlowDown, &keys, &mouse) {
        let mut amt = velocity.linvel * 0.1;
        if amt.dot(amt) > max_speed * max_speed {
            amt = amt.normalize() * max_speed;
        }
        velocity.linvel -= amt;
    }

    let y = velocity.linvel.y;

    velocity.linvel.y = 0.0;

    if velocity.linvel.dot(velocity.linvel.clone()) > max_speed * max_speed {
        velocity.linvel = velocity.linvel.normalize() * max_speed;
    }

    velocity.linvel.y = y;
}

fn create_sun(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn_bundle(PointLightBundle {
            transform: Transform::from_xyz(0.0, 100.0, 0.0),
            point_light: PointLight {
                intensity: 160000.0,
                range: 16000.0,
                color: Color::WHITE,
                shadows_enabled: true,
                ..default()
            },
            ..default()
        })
        .with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.1,
                    ..default()
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::RED,
                    emissive: Color::rgba_linear(100.0, 0.0, 0.0, 0.0),
                    ..default()
                }),
                ..default()
            });
        });
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let host_name = if args.len() > 1 {
        args.get(1).unwrap().to_owned()
    } else {
        get_local_ipaddress().expect("127.0.0.1").to_owned()
    };

    println!("Host: {}", host_name);

    let mut app = App::new();

    app.insert_resource(ConnectionConfig {
        host_name: host_name.into(),
    });

    game_state::register(&mut app);

    app.insert_resource(ImageSettings::default_nearest()) // MUST be before default plugins!
        .add_plugins(CosmosCorePluginGroup::default())
        .add_plugins(ClientPluginGroup::default())
        .add_plugin(RenetClientPlugin {})
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_system_set(
            SystemSet::on_enter(GameState::Connecting).with_system(connect::establish_connection),
        )
        .add_system_set(
            SystemSet::on_update(GameState::Connecting).with_system(connect::wait_for_connection),
        )
        .add_system_set(SystemSet::on_enter(GameState::LoadingWorld).with_system(create_sun))
        .add_system_set(
            SystemSet::on_update(GameState::LoadingWorld)
                .with_system(connect::wait_for_done_loading)
                .with_system(monitor_block_updates_system)
                .with_system(listen_for_structure_event)
                .with_system(listen_for_new_physics_event),
        )
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(process_player_movement)
                .with_system(monitor_block_updates_system)
                .with_system(listen_for_structure_event)
                .with_system(listen_for_new_physics_event),
        );

    inputs::register(&mut app);
    window::setup::register(&mut app);
    asset_loading::register(&mut app);
    events::register(&mut app);
    block_interactions::register(&mut app);
    chunk_retreiver::register(&mut app);
    camera_controller::register(&mut app);
    crosshair::register(&mut app);
    receiver::register(&mut app);
    structure_renderer::register(&mut app);
    sync::register(&mut app);

    app.run();
}
