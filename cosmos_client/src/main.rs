pub mod asset;
pub mod camera;
pub mod events;
pub mod input;
pub mod interactions;
pub mod lang;
pub mod netty;
pub mod plugin;
pub mod rendering;
pub mod state;
pub mod structure;
pub mod ui;
pub mod window;

use std::env;
use std::f32::consts::PI;

// use bevy_rapier3d::render::RapierDebugRenderPlugin;
use bevy_renet::renet::RenetClient;
use camera::camera_controller;
use cosmos_core::entities::player::Player;
use cosmos_core::events::structure::change_pilot_event::ChangePilotEvent;
use cosmos_core::netty::client_reliable_messages::ClientReliableMessages;
use cosmos_core::netty::client_unreliable_messages::ClientUnreliableMessages;
use cosmos_core::netty::{get_local_ipaddress, NettyChannel};
use cosmos_core::structure::ship::pilot::Pilot;
use cosmos_core::structure::ship::ship_movement::ShipMovement;
use input::inputs::{self, CosmosInputHandler, CosmosInputs};
use interactions::block_interactions;
use netty::connect::{self, ConnectionConfig};
use netty::flags::LocalPlayer;
use netty::mapping::NetworkMapping;
use rendering::structure_renderer;
use state::game_state::GameState;
use structure::chunk_retreiver;
use ui::crosshair::CrosshairOffset;
use window::setup::DeltaCursorPosition;

use crate::plugin::client_plugin::ClientPluginGroup;
use crate::rendering::structure_renderer::monitor_block_updates_system;
use crate::rendering::uv_mapper::UVMapper;
use bevy::prelude::*;
use bevy_rapier3d::prelude::{RapierConfiguration, Vect, Velocity};
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
    wnd: Res<Windows>,
) {
    if query.get_single().is_ok() {
        let mut movement = ShipMovement::default();

        if input_handler.check_pressed(CosmosInputs::MoveForward, &keys, &mouse) {
            movement.movement.z += 1.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveBackward, &keys, &mouse) {
            movement.movement.z -= 1.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveUpOrJump, &keys, &mouse) {
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

        if input_handler.check_just_pressed(CosmosInputs::StopPiloting, &keys, &mouse) {
            client.send_message(
                NettyChannel::Reliable.id(),
                bincode::serialize(&ClientReliableMessages::StopPiloting).unwrap(),
            );
        }

        let w = wnd.primary();
        let hw = w.width() / 2.0;
        let hh = w.height() / 2.0;
        let p2 = PI / 2.0;

        crosshair_offset.x += cursor_delta_position.x;
        crosshair_offset.y += cursor_delta_position.y;

        movement.torque = Vec3::new(
            crosshair_offset.y / hh * p2,
            -crosshair_offset.x / hw * p2,
            0.0,
        );

        client.send_message(
            NettyChannel::Unreliable.id(),
            bincode::serialize(&ClientUnreliableMessages::SetMovement { movement }).unwrap(),
        );
    }
}

fn reset_cursor(
    mut event_reader: EventReader<ChangePilotEvent>,
    query: Query<&LocalPlayer>,
    pilot_query: Query<&Pilot>,
    mut crosshair_position: ResMut<CrosshairOffset>,
) {
    for ev in event_reader.iter() {
        if let Some(pilot) = ev.pilot_entity {
            if query.get(pilot).is_ok() {
                crosshair_position.x = 0.0;
                crosshair_position.y = 0.0;
            }
        } else if let Ok(pilot) = pilot_query.get(ev.structure_entity) {
            if query.get(pilot.entity).is_ok() {
                crosshair_position.x = 0.0;
                crosshair_position.y = 0.0;
            }
        }
    }
}

fn sync_pilot_to_ship(mut query: Query<&mut Transform, (With<Player>, With<Pilot>)>) {
    for mut trans in query.iter_mut() {
        trans.translation.x = 0.0;
        trans.translation.y = 0.0;
        trans.translation.z = 0.0;
        trans.rotation = Quat::IDENTITY;
    }
}

fn process_player_movement(
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    time: Res<Time>,
    input_handler: ResMut<CosmosInputHandler>,
    mut query: Query<&mut Velocity, (With<LocalPlayer>, Without<Pilot>)>,
    cam_query: Query<&Transform, With<Camera>>,
) {
    // This will be err if the player is piloting a ship
    if let Ok(mut velocity) = query.get_single_mut() {
        let cam_trans = cam_query.single();

        let max_speed: f32 = match input_handler.check_pressed(CosmosInputs::Sprint, &keys, &mouse)
        {
            false => 5.0,
            true => 20.0,
        };

        let mut forward = cam_trans.forward();
        let mut right = cam_trans.right();
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

        if velocity.linvel.dot(velocity.linvel) > max_speed * max_speed {
            velocity.linvel = velocity.linvel.normalize() * max_speed;
        }

        velocity.linvel.y = y;
    }
}

fn create_sun(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn(PointLightBundle {
            transform: Transform::from_xyz(0.0, 100.0, 0.0),
            point_light: PointLight {
                intensity: 160000.0,
                range: 160000.0,
                color: Color::WHITE,
                shadows_enabled: true,
                ..default()
            },
            ..default()
        })
        .with_children(|builder| {
            builder.spawn(PbrBundle {
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
        get_local_ipaddress()
    };

    println!("Host: {host_name}");

    let mut app = App::new();

    app.insert_resource(ConnectionConfig { host_name });

    app.insert_resource(RapierConfiguration {
        gravity: Vec3::ZERO,
        ..default()
    })
    .add_state(GameState::PreLoading)
    .add_plugins(CosmosCorePluginGroup::new(
        GameState::PreLoading,
        GameState::Loading,
        GameState::PostLoading,
        GameState::Connecting,
        GameState::Playing,
    ))
    .add_plugins(ClientPluginGroup::default())
    .add_plugin(RenetClientPlugin::default())
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
            .with_system(monitor_block_updates_system),
    )
    .add_system_set(
        SystemSet::on_update(GameState::Playing)
            .with_system(process_player_movement)
            .with_system(process_ship_movement)
            .with_system(monitor_block_updates_system)
            .with_system(reset_cursor)
            .with_system(sync_pilot_to_ship),
    );

    inputs::register(&mut app);
    window::setup::register(&mut app);
    asset::register(&mut app);
    events::register(&mut app);
    block_interactions::register(&mut app);
    chunk_retreiver::register(&mut app);
    camera_controller::register(&mut app);
    ui::register(&mut app);
    netty::register(&mut app);
    structure_renderer::register(&mut app);
    lang::register(&mut app);
    structure::register(&mut app);

    app.run();
}
