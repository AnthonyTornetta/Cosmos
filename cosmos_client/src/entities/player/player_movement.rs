//! Handles the basic player movement while walking around. This is not responsible for piloting ships. See [`ship_movement`] for that.

use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::structure::ship::pilot::Pilot;

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    netty::flags::LocalPlayer,
    rendering::MainCamera,
    state::game_state::GameState,
    structure::planet::align_player::{self, PlayerAlignment},
};

fn process_player_movement(
    time: Res<Time>,
    input_handler: InputChecker,
    mut query: Query<(Entity, &mut Velocity, &Transform, Option<&PlayerAlignment>), (With<LocalPlayer>, Without<Pilot>)>,
    cam_query: Query<&Transform, With<MainCamera>>,
    parent_query: Query<&Parent>,
    global_transform_query: Query<&GlobalTransform>,
) {
    // This will be err if the player is piloting a ship
    if let Ok((ent, mut velocity, player_transform, player_alignment)) = query.get_single_mut() {
        let cam_trans = player_transform.mul_transform(*cam_query.single());

        let max_speed: f32 = match input_handler.check_pressed(CosmosInputs::Sprint) {
            false => 3.0,
            true => 20.0,
        };

        // All relative to player
        let mut forward = cam_trans.forward();
        let mut right = cam_trans.right();
        let up = player_transform.up();

        if let Some(player_alignment) = player_alignment {
            match player_alignment.0 {
                align_player::Axis::X => {
                    forward.x = 0.0;
                    right.x = 0.0;
                }
                align_player::Axis::Y => {
                    forward.y = 0.0;
                    right.y = 0.0;
                }
                align_player::Axis::Z => {
                    forward.z = 0.0;
                    right.z = 0.0;
                }
            }
        }

        forward = forward.normalize_or_zero() * 100.0;
        right = right.normalize_or_zero() * 100.0;
        let movement_up = up * 2.0;

        let time = time.delta_seconds();

        let parent_rot = parent_query
            .get(ent)
            .map(|p| {
                global_transform_query
                    .get(p.get())
                    .map(|x| Quat::from_affine3(&x.affine()))
                    .unwrap_or(Quat::IDENTITY)
            })
            .unwrap_or(Quat::IDENTITY);

        let mut new_linvel = parent_rot.inverse().mul_vec3(velocity.linvel);

        if input_handler.check_pressed(CosmosInputs::MoveForward) {
            new_linvel += forward * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveBackward) {
            new_linvel -= forward * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveUp) {
            new_linvel += movement_up * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveDown) {
            new_linvel -= movement_up * time;
        }
        if input_handler.check_just_pressed(CosmosInputs::Jump) {
            new_linvel += up * 5.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveLeft) {
            new_linvel -= right * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveRight) {
            new_linvel += right * time;
        }
        if input_handler.check_pressed(CosmosInputs::SlowDown) {
            let mut amt = new_linvel * 0.5;
            if amt.dot(amt) > max_speed * max_speed {
                amt = amt.normalize() * max_speed;
            }
            new_linvel -= amt;
        }

        if let Some(player_alignment) = player_alignment {
            match player_alignment.0 {
                align_player::Axis::X => {
                    let x = new_linvel.x;

                    new_linvel.x = 0.0;

                    if new_linvel.dot(new_linvel) > max_speed * max_speed {
                        new_linvel = new_linvel.normalize() * max_speed;
                    }

                    new_linvel.x = x;
                }
                align_player::Axis::Y => {
                    let y = new_linvel.y;

                    new_linvel.y = 0.0;

                    if new_linvel.dot(new_linvel) > max_speed * max_speed {
                        new_linvel = new_linvel.normalize() * max_speed;
                    }

                    new_linvel.y = y;
                }
                align_player::Axis::Z => {
                    let z = new_linvel.z;

                    new_linvel.z = 0.0;

                    if new_linvel.dot(new_linvel) > max_speed * max_speed {
                        new_linvel = new_linvel.normalize() * max_speed;
                    }

                    new_linvel.z = z;
                }
            }
        } else if new_linvel.dot(new_linvel) > max_speed * max_speed {
            new_linvel = new_linvel.normalize() * max_speed;
        }

        velocity.linvel = parent_rot.mul_vec3(new_linvel);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, process_player_movement.run_if(in_state(GameState::Playing)));
}
