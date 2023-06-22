//! Contains all the logic for the client-side of Cosmos.

#![warn(missing_docs)]

pub mod asset;
pub mod block;
pub mod camera;
pub mod entities;
pub mod events;
pub mod input;
pub mod interactions;
pub mod inventory;
pub mod lang;
pub mod loading;
pub mod materials;
pub mod netty;
pub mod physics;
pub mod plugin;
pub mod projectiles;
pub mod rendering;
pub mod skybox;
pub mod state;
pub mod structure;
pub mod ui;
pub mod universe;
pub mod window;

use std::env;
use std::f32::consts::PI;

use bevy::window::PrimaryWindow;
use bevy_renet::renet::RenetClient;
use bevy_renet::transport::NetcodeClientPlugin;
use cosmos_core::netty::client_reliable_messages::ClientReliableMessages;
use cosmos_core::netty::client_unreliable_messages::ClientUnreliableMessages;
use cosmos_core::netty::{cosmos_encoder, get_local_ipaddress, NettyChannelClient};
use cosmos_core::structure::ship::pilot::Pilot;
use cosmos_core::structure::ship::ship_movement::ShipMovement;
use input::inputs::{CosmosInputHandler, CosmosInputs};
use netty::connect::{self, HostConfig};
use netty::flags::LocalPlayer;
use netty::mapping::NetworkMapping;
use rendering::MainCamera;
use state::game_state::GameState;
use structure::planet::align_player::PlayerAlignment;
use ui::crosshair::CrosshairOffset;
use window::setup::DeltaCursorPosition;

use bevy::prelude::*;
use bevy_rapier3d::prelude::{RapierConfiguration, TimestepMode, Velocity};
use bevy_renet::RenetClientPlugin;
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;

fn process_ship_movement(
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    input_handler: ResMut<CosmosInputHandler>,
    query: Query<Entity, (With<LocalPlayer>, With<Pilot>)>,
    mut client: ResMut<RenetClient>,
    mut crosshair_offset: ResMut<CrosshairOffset>,
    cursor_delta_position: Res<DeltaCursorPosition>,
    primary_query: Query<&Window, With<PrimaryWindow>>,
) {
    if query.get_single().is_ok() {
        let mut movement = ShipMovement::default();

        if input_handler.check_pressed(CosmosInputs::MoveForward, &keys, &mouse) {
            movement.movement.z += 1.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveBackward, &keys, &mouse) {
            movement.movement.z -= 1.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveUp, &keys, &mouse) {
            movement.movement.y += 1.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveDown, &keys, &mouse) {
            movement.movement.y -= 1.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveLeft, &keys, &mouse) {
            movement.movement.x -= 1.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveRight, &keys, &mouse) {
            movement.movement.x += 1.0;
        }

        movement.braking = input_handler.check_pressed(CosmosInputs::SlowDown, &keys, &mouse);

        if input_handler.check_just_pressed(CosmosInputs::StopPiloting, &keys, &mouse) {
            client.send_message(
                NettyChannelClient::Reliable,
                cosmos_encoder::serialize(&ClientReliableMessages::StopPiloting),
            );
        }

        let w = primary_query.get_single().expect("Missing primary window!");
        let hw = w.width() / 2.0;
        let hh = w.height() / 2.0;
        let p2 = PI / 2.0; // 45 deg (half of FOV)

        let max_w = hw * 0.9;
        let max_h = hh * 0.9;

        // Prevents you from moving cursor off screen
        // Reduces cursor movement the closer you get to edge of screen until it reaches 0 at hw/2 or hh/2
        crosshair_offset.x += cursor_delta_position.x
            - (cursor_delta_position.x * (crosshair_offset.x.abs() / max_w));
        crosshair_offset.y += cursor_delta_position.y
            - (cursor_delta_position.y * (crosshair_offset.y.abs() / max_h));

        crosshair_offset.x = crosshair_offset.x.clamp(-hw, hw);
        crosshair_offset.y = crosshair_offset.y.clamp(-hh, hh);

        let mut roll = 0.0;

        if input_handler.check_pressed(CosmosInputs::RollLeft, &keys, &mouse) {
            roll += 0.25;
        }
        if input_handler.check_pressed(CosmosInputs::RollRight, &keys, &mouse) {
            roll -= 0.25;
        }

        movement.torque = Vec3::new(
            crosshair_offset.y / hh * p2 / 2.0,
            -crosshair_offset.x / hw * p2 / 2.0,
            roll,
        );

        client.send_message(
            NettyChannelClient::Unreliable,
            cosmos_encoder::serialize(&ClientUnreliableMessages::SetMovement { movement }),
        );
    }
}

fn reset_cursor(
    local_player_without_pilot: Query<(), (With<LocalPlayer>, Without<Pilot>)>,
    mut crosshair_position: ResMut<CrosshairOffset>,
) {
    if !local_player_without_pilot.is_empty() {
        crosshair_position.x = 0.0;
        crosshair_position.y = 0.0;
    }
}

fn process_player_movement(
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    time: Res<Time>,
    input_handler: ResMut<CosmosInputHandler>,
    mut query: Query<
        (&mut Velocity, &Transform, Option<&PlayerAlignment>),
        (With<LocalPlayer>, Without<Pilot>),
    >,
    cam_query: Query<&GlobalTransform, With<MainCamera>>,
) {
    // This will be err if the player is piloting a ship
    if let Ok((mut velocity, player_transform, player_alignment)) = query.get_single_mut() {
        let cam_trans = cam_query.single();

        let max_speed: f32 = match input_handler.check_pressed(CosmosInputs::Sprint, &keys, &mouse)
        {
            false => 3.0,
            true => 20.0,
        };

        let mut forward = cam_trans.forward();
        let mut right = cam_trans.right();
        let up = player_transform.up();

        if let Some(player_alignment) = player_alignment {
            match player_alignment.0 {
                structure::planet::align_player::Axis::X => {
                    forward.x = 0.0;
                    right.x = 0.0;
                }
                structure::planet::align_player::Axis::Y => {
                    forward.y = 0.0;
                    right.y = 0.0;
                }
                structure::planet::align_player::Axis::Z => {
                    forward.z = 0.0;
                    right.z = 0.0;
                }
            }
        }

        println!("Forward: {forward}");

        forward = forward.normalize_or_zero() * 100.0;
        right = right.normalize_or_zero() * 100.0;
        let movement_up = up * 2.0;

        let time = time.delta_seconds();

        if input_handler.check_pressed(CosmosInputs::MoveForward, &keys, &mouse) {
            velocity.linvel += forward * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveBackward, &keys, &mouse) {
            velocity.linvel -= forward * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveUp, &keys, &mouse) {
            velocity.linvel += movement_up * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveDown, &keys, &mouse) {
            velocity.linvel -= movement_up * time;
        }
        if input_handler.check_just_pressed(CosmosInputs::Jump, &keys, &mouse) {
            velocity.linvel += up * 5.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveLeft, &keys, &mouse) {
            velocity.linvel -= right * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveRight, &keys, &mouse) {
            velocity.linvel += right * time;
        }
        if input_handler.check_pressed(CosmosInputs::SlowDown, &keys, &mouse) {
            let mut amt = velocity.linvel * 0.5;
            if amt.dot(amt) > max_speed * max_speed {
                amt = amt.normalize() * max_speed;
            }
            velocity.linvel -= amt;
        }

        if let Some(player_alignment) = player_alignment {
            match player_alignment.0 {
                structure::planet::align_player::Axis::X => {
                    let x = velocity.linvel.x;

                    velocity.linvel.x = 0.0;

                    if velocity.linvel.dot(velocity.linvel) > max_speed * max_speed {
                        velocity.linvel = velocity.linvel.normalize() * max_speed;
                    }

                    velocity.linvel.x = x;
                }
                structure::planet::align_player::Axis::Y => {
                    let y = velocity.linvel.y;

                    velocity.linvel.y = 0.0;

                    if velocity.linvel.dot(velocity.linvel) > max_speed * max_speed {
                        velocity.linvel = velocity.linvel.normalize() * max_speed;
                    }

                    velocity.linvel.y = y;
                }
                structure::planet::align_player::Axis::Z => {
                    let z = velocity.linvel.z;

                    velocity.linvel.z = 0.0;

                    if velocity.linvel.dot(velocity.linvel) > max_speed * max_speed {
                        velocity.linvel = velocity.linvel.normalize() * max_speed;
                    }

                    velocity.linvel.z = z;
                }
            }
        } else if velocity.linvel.dot(velocity.linvel) > max_speed * max_speed {
            velocity.linvel = velocity.linvel.normalize() * max_speed;
        }
    }
}

fn create_sun(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 30000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::ZERO,
            rotation: Quat::from_euler(EulerRot::XYZ, -PI / 4.0, 0.1, 0.1),
            ..default()
        },
        ..default()
    });
}

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
        .add_plugin(RenetClientPlugin)
        .add_plugin(NetcodeClientPlugin)
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_systems((
            connect::establish_connection.in_schedule(OnEnter(GameState::Connecting)),
            connect::wait_for_connection.in_set(OnUpdate(GameState::Connecting)),
        ))
        .add_system(create_sun.in_schedule(OnEnter(GameState::LoadingWorld)))
        .add_system(connect::wait_for_done_loading.in_set(OnUpdate(GameState::LoadingWorld)))
        .add_systems(
            (process_player_movement, process_ship_movement, reset_cursor)
                .in_set(OnUpdate(GameState::Playing)),
        );

    input::register(&mut app);
    window::register(&mut app);
    asset::register(&mut app);
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
    physics::register(&mut app);

    app.run();
}
