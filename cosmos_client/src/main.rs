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
pub mod plugin;
pub mod projectiles;
pub mod rendering;
pub mod state;
pub mod structure;
pub mod ui;
pub mod window;

use std::env;
use std::f32::consts::{E, PI, TAU};

use bevy::window::PrimaryWindow;
// use bevy_rapier3d::render::RapierDebugRenderPlugin;
use bevy_renet::renet::RenetClient;
use camera::camera_controller;
use cosmos_core::entities::player::Player;
use cosmos_core::events::structure::change_pilot_event::ChangePilotEvent;
use cosmos_core::netty::client_reliable_messages::ClientReliableMessages;
use cosmos_core::netty::client_unreliable_messages::ClientUnreliableMessages;
use cosmos_core::netty::{cosmos_encoder, get_local_ipaddress, NettyChannel};
use cosmos_core::physics::location::Location;
use cosmos_core::structure::ship::pilot::Pilot;
use cosmos_core::structure::ship::ship_movement::ShipMovement;
use input::inputs::{self, CosmosInputHandler, CosmosInputs};
use interactions::block_interactions;
use netty::connect::{self, ConnectionConfig};
use netty::flags::LocalPlayer;
use netty::mapping::NetworkMapping;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use state::game_state::GameState;
use structure::chunk_retreiver;
use ui::crosshair::CrosshairOffset;
use window::setup::DeltaCursorPosition;

use bevy::prelude::*;
use bevy_rapier3d::prelude::{RapierConfiguration, TimestepMode, Vect, Velocity};
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
                NettyChannel::Reliable.id(),
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
            NettyChannel::Unreliable.id(),
            cosmos_encoder::serialize(&ClientUnreliableMessages::SetMovement { movement }),
        );
    }
}

fn reset_cursor(
    mut event_reader: EventReader<ChangePilotEvent>,
    local_player_query: Query<&LocalPlayer>,
    pilot_query: Query<&Pilot>,
    mut crosshair_position: ResMut<CrosshairOffset>,
) {
    for ev in event_reader.iter() {
        if let Some(pilot) = ev.pilot_entity {
            if local_player_query.get(pilot).is_ok() {
                crosshair_position.x = 0.0;
                crosshair_position.y = 0.0;
            }
        } else if let Ok(pilot) = pilot_query.get(ev.structure_entity) {
            if local_player_query.get(pilot.entity).is_ok() {
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
            false => 3.0,
            true => 20.0,
        };

        let mut forward = cam_trans.forward();
        let mut right = cam_trans.right();
        let up = Vect::new(0.0, 1.0, 0.0);

        forward.y = 0.0;
        right.y = 0.0;

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

        let y = velocity.linvel.y;

        velocity.linvel.y = 0.0;

        if velocity.linvel.dot(velocity.linvel) > max_speed * max_speed {
            velocity.linvel = velocity.linvel.normalize() * max_speed;
        }

        velocity.linvel.y = y;
    }
}

// Calculates the distance from the origin of a spiral arm given an angle.
fn spiral_function(theta: f32) -> f32 {
    E.powf(theta / 2.0)
}

// Calculates what offset must be necessary for spiral_function to output r given the angle (theta - offset).
// Update this whenever spiral_function is changed.
fn inverse_spiral_function(r: f32, theta: f32) -> f32 {
    theta - 2.0 * r.ln()
}

fn distance_from_star_spiral(x: f32, y: f32) -> f32 {
    // Number of spiral arms in the galaxy.
    let num_spirals: f32 = 8.0;

    let r: f32 = (x * x + y * y).sqrt();
    if r.abs() < 0.0001 {
        // Origin case, trig math gets messed up, but all arms are equally close anyways.
        return spiral_function(0.0);
    }
    let theta: f32 = y.atan2(x);

    let offset: f32 = inverse_spiral_function(r, theta);
    let spiral_index: f32 = (offset * num_spirals / TAU).round();
    let spiral_offset: f32 = spiral_index * TAU / num_spirals;

    (spiral_function(theta - spiral_offset) - r).abs() * (r / 4.0)
}

fn create_sun(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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

    const MULT: f32 = 3.0;

    let seed: u64 = rand::random();

    let min = -22.0 * MULT;
    let max = 22.0 * MULT;

    let mut at_z = min;
    while at_z <= max {
        let mut at_x = min;

        while at_x <= max {
            let seed_x = (at_x + max + 2.0) as u64;
            let seed_z = (at_z + max + 2.0) as u64;

            let local_seed = seed
                .wrapping_mul(seed_x)
                .wrapping_add(seed_z)
                .wrapping_mul(seed_z)
                .wrapping_sub(seed_x);

            let mut rng = ChaCha8Rng::seed_from_u64(local_seed);

            let distance = distance_from_star_spiral(at_x / MULT, at_z / MULT);

            let prob = 1.0 / (distance * distance);
            let num = rng.gen_range(0..10_000) as f32 / 10_000.0;

            if num < prob {
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::UVSphere {
                            radius: 0.1,
                            ..default()
                        })),
                        material: materials.add(StandardMaterial {
                            base_color: Color::WHITE,
                            emissive: Color::rgba_linear(100.0, 0.0, 0.0, 0.0),
                            ..default()
                        }),
                        transform: Transform::from_xyz(at_x, 0.0, at_z),
                        ..default()
                    },
                    Location::new(Vec3::new(at_x, 0.0, at_z), 0, 0, 0),
                ));

                println!("spawned one at {at_x}, {at_z}");
            }

            at_x += 1.0;
        }

        at_z += 1.0;
    }
    // commands
    //     .spawn(PointLightBundle {
    //         transform: Transform::from_xyz(0.0, 100.0, 0.0),
    //         point_light: PointLight {
    //             intensity: 160000.0,
    //             range: 160000.0,
    //             color: Color::WHITE,
    //             shadows_enabled: true,
    //             ..default()
    //         },
    //         ..default()
    //     })
    //     .with_children(|builder| {
    //         builder.spawn(PbrBundle {
    //             mesh: meshes.add(Mesh::from(shape::UVSphere {
    //                 radius: 0.1,
    //                 ..default()
    //             })),
    //             material: materials.add(StandardMaterial {
    //                 base_color: Color::RED,
    //                 emissive: Color::rgba_linear(100.0, 0.0, 0.0, 0.0),
    //                 ..default()
    //             }),
    //             ..default()
    //         });
    //     });

    // commands
    //     .spawn(PointLightBundle {
    //         transform: Transform::from_xyz(0.5, 2.5, 0.5),
    //         point_light: PointLight {
    //             intensity: 600.0,
    //             range: 20.0,
    //             color: Color::WHITE,
    //             radius: 0.6,
    //             shadows_enabled: true,
    //             ..default()
    //         },
    //         ..default()
    //     })
    //     .with_children(|builder| {
    //         builder.spawn(PbrBundle {
    //             mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
    //             material: materials.add(StandardMaterial {
    //                 base_color: Color::WHITE,
    //                 emissive: Color::rgba_linear(1.0, 1.0, 1.0, 0.0),
    //                 ..default()
    //             }),
    //             ..default()
    //         });
    //     });
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

    app.insert_resource(ConnectionConfig { host_name })
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
        .add_state::<GameState>()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(CosmosCorePluginGroup::new(
            GameState::PreLoading,
            GameState::Loading,
            GameState::PostLoading,
            GameState::Connecting,
            GameState::Playing,
        ))
        .add_plugin(RenetClientPlugin::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_systems((
            connect::establish_connection.in_schedule(OnEnter(GameState::Connecting)),
            connect::wait_for_connection.in_set(OnUpdate(GameState::Connecting)),
        ))
        .add_system(create_sun.in_schedule(OnEnter(GameState::LoadingWorld)))
        .add_system(connect::wait_for_done_loading.in_set(OnUpdate(GameState::LoadingWorld)))
        .add_systems(
            (
                process_player_movement,
                process_ship_movement,
                reset_cursor,
                sync_pilot_to_ship,
            )
                .in_set(OnUpdate(GameState::Playing)),
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
    lang::register(&mut app);
    structure::register(&mut app);
    block::register(&mut app);
    projectiles::register(&mut app);
    materials::register(&mut app);
    loading::register(&mut app);
    entities::register(&mut app);
    inventory::register(&mut app);
    rendering::register(&mut app);

    app.run();
}
