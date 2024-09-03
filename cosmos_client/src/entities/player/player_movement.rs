//! Handles the basic player movement while walking around. This is not responsible for piloting ships. See [`ship_movement`] for that.

use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    physics::location::LocationPhysicsSet,
    projectiles::laser::LaserSystemSet,
    structure::{shared::build_mode::BuildMode, ship::pilot::Pilot},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    rendering::MainCamera,
    state::game_state::GameState,
    structure::planet::align_player::{self, PlayerAlignment},
    ui::components::show_cursor::ShowCursor,
};

pub(crate) fn process_player_movement(
    time: Res<Time>,
    input_handler: InputChecker,
    mut query: Query<
        (Entity, &mut Velocity, &GlobalTransform, Option<&PlayerAlignment>),
        (With<LocalPlayer>, Without<Pilot>, Without<BuildMode>),
    >,
    cam_query: Query<&Transform, With<MainCamera>>,
    parent_query: Query<&Parent>,
    q_global_transform: Query<&GlobalTransform>,
    q_show_cursor: Query<(), With<ShowCursor>>,

    q_camera_trans: Query<&GlobalTransform, With<MainCamera>>,
) {
    let any_open_menus = !q_show_cursor.is_empty();

    let Ok(cam_trans) = cam_query.get_single() else {
        return;
    };

    // This will be err if the player is piloting a ship
    if let Ok((ent, mut velocity, player_transform, player_alignment)) = query.get_single_mut() {
        let max_speed: f32 = if !any_open_menus && input_handler.check_pressed(CosmosInputs::Sprint) {
            20.0
        } else {
            3.0
        };

        // All relative to player
        // let mut forward = *cam_g_trans.forward();
        // let mut right = *cam_g_trans.right();
        // let up = *player_transform.up();

        let player_rot = Quat::from_affine3(&player_transform.affine());
        let player_inv_rot = player_rot.inverse();

        // forward = player_inv_rot * forward;
        // forward.y = 0.0;
        // forward = player_rot * forward;
        // right = player_inv_rot * right;
        // right.y = 0.0;
        // right = player_rot * right;

        let mut forward = *cam_trans.forward(); //Vec3::NEG_Z;
        let mut right = *cam_trans.right();
        let up = Vec3::Y;

        forward.y = 0.0;
        right.y = 0.0;

        // if let Some(player_alignment) = player_alignment {
        //     let aligned_rot = player_alignment
        //         .aligned_to
        //         .map(|x| q_global_transform.get(x).ok().map(|x| Quat::from_affine3(&x.affine())))
        //         .flatten()
        //         .unwrap_or(Quat::IDENTITY);
        //
        //     let inverse = aligned_rot.inverse();
        //     forward = inverse * forward;
        //     right = inverse * right;
        //
        //     match player_alignment.axis {
        //         align_player::Axis::X => {
        //             forward.x = 0.0;
        //             right.x = 0.0;
        //         }
        //         align_player::Axis::Y => {
        //             forward.y = 0.0;
        //             right.y = 0.0;
        //         }
        //         align_player::Axis::Z => {
        //             forward.z = 0.0;
        //             right.z = 0.0;
        //         }
        //     }
        //     let aligned_rot = aligned_rot; //.inverse();
        //     forward = aligned_rot * forward;
        //     right = aligned_rot * right;
        // }

        forward = forward.normalize_or_zero() * 100.0;
        right = right.normalize_or_zero() * 100.0;
        let movement_up = up.normalize_or_zero() * 2.0;

        let time = time.delta_seconds();
        //
        // let parent_rot = parent_query
        //     .get(ent)
        //     .map(|x| Some(x.get()))
        //     .unwrap_or(player_alignment.map(|x| x.aligned_to).unwrap_or(None))
        //     .map(|p| {
        //         q_global_transform
        //             .get(p)
        //             .map(|x| Quat::from_affine3(&x.affine()))
        //             .unwrap_or(Quat::IDENTITY)
        //     })
        //     .unwrap_or(Quat::IDENTITY);
        //

        let mut new_linvel = player_inv_rot * velocity.linvel; //parent_rot.inverse().mul_vec3(velocity.linvel);

        if !any_open_menus {
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
        }
        //
        // if let Some(player_alignment) = player_alignment {
        //     match player_alignment.axis {
        //         align_player::Axis::X => {
        //             let x = new_linvel.x;
        //
        //             new_linvel.x = 0.0;
        //
        //             if new_linvel.dot(new_linvel) > max_speed * max_speed {
        //                 new_linvel = new_linvel.normalize() * max_speed;
        //             }
        //
        //             new_linvel.x = x;
        //         }
        //         align_player::Axis::Y => {
        //             let y = new_linvel.y;
        //
        //             new_linvel.y = 0.0;
        //
        //             if new_linvel.dot(new_linvel) > max_speed * max_speed {
        //                 new_linvel = new_linvel.normalize() * max_speed;
        //             }
        //
        //             new_linvel.y = y;
        //         }
        //         align_player::Axis::Z => {
        //             let z = new_linvel.z;
        //
        //             new_linvel.z = 0.0;
        //
        //             if new_linvel.dot(new_linvel) > max_speed * max_speed {
        //                 new_linvel = new_linvel.normalize() * max_speed;
        //             }
        //
        //             new_linvel.z = z;
        //         }
        //     }
        // } else if new_linvel.dot(new_linvel) > max_speed * max_speed {
        //     new_linvel = new_linvel.normalize() * max_speed;
        // }
        if player_alignment.is_some() {
            let y = new_linvel.y;

            new_linvel.y = 0.0;

            if new_linvel.dot(new_linvel) > max_speed * max_speed {
                new_linvel = new_linvel.normalize_or_zero() * max_speed;
            }

            new_linvel.y = y;
        } else if new_linvel.dot(new_linvel) > max_speed * max_speed {
            new_linvel = new_linvel.normalize_or_zero() * max_speed;
        }

        velocity.linvel = player_rot * new_linvel; //parent_rot.mul_vec3(new_linvel);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Player movement inputs are handled and applied to their velocity
pub enum PlayerMovementSet {
    /// Player movement inputs are handled and applied to their velocity
    ProcessPlayerMovement,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        PlayerMovementSet::ProcessPlayerMovement.before(LocationPhysicsSet::DoPhysics),
    );

    app.add_systems(
        Update,
        process_player_movement
            .ambiguous_with(LaserSystemSet::SendHitEvents)
            .in_set(NetworkingSystemsSet::Between)
            .in_set(PlayerMovementSet::ProcessPlayerMovement)
            .run_if(in_state(GameState::Playing)),
    );
}
