//! Handles the basic player movement while walking around. This is not responsible for piloting ships. See [`ship_movement`] for that.

use bevy::prelude::*;
use bevy_rapier3d::{
    plugin::{RapierContextEntityLink, ReadRapierContext},
    prelude::{ActiveEvents, Collider, Sensor, Velocity},
};
use cosmos_core::{
    block::specific_blocks::gravity_well::GravityWell,
    ecs::{compute_totally_accurate_global_transform, sets::FixedUpdateSet},
    netty::client::LocalPlayer,
    physics::location::LocationPhysicsSet,
    prelude::Planet,
    projectiles::laser::LaserSystemSet,
    state::GameState,
    structure::{shared::build_mode::BuildMode, ship::pilot::Pilot},
};

use crate::{
    camera::camera_controller::CameraHelper,
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    rendering::MainCamera,
    structure::planet::align_player::PlayerAlignment,
    ui::components::show_cursor::ShowCursor,
};

#[derive(Component, Debug)]
/// Indicates if the player is touching the ground
pub struct Grounded;

#[derive(Component)]
struct GroundedChecker;

fn append_grounded_check(mut commands: Commands, q_player: Query<Entity, Added<LocalPlayer>>) {
    let Ok(player_ent) = q_player.single() else {
        return;
    };
    commands.entity(player_ent).with_children(|p| {
        p.spawn((
            GroundedChecker,
            Visibility::default(),
            Transform::from_xyz(0.0, -0.80, 0.0),
            Name::new("Ground checker"),
            Collider::cuboid(0.1, 0.1, 0.1),
            Sensor,
            ActiveEvents::COLLISION_EVENTS,
        ));
    });
}

fn check_grounded(
    mut commands: Commands,
    rapier_context: ReadRapierContext,
    q_player: Query<Entity, With<LocalPlayer>>,
    q_ground_checker: Query<(&RapierContextEntityLink, Entity), With<GroundedChecker>>,
) {
    let Ok((rapier_link, collider_ent)) = q_ground_checker.single() else {
        return;
    };

    let Ok(player_ent) = q_player.single() else {
        return;
    };

    let context = rapier_context.get(*rapier_link);

    let touching_ground = context.intersection_pairs_with(collider_ent).any(|x| x.2);
    if touching_ground {
        commands.entity(player_ent).insert(Grounded);
    } else {
        commands.entity(player_ent).remove::<Grounded>();
    }
}

fn process_player_movement(
    time: Res<Time>,
    input_handler: InputChecker,
    mut q_local_player: Query<
        (
            &mut Velocity,
            &GlobalTransform,
            Option<&PlayerAlignment>,
            Has<Grounded>,
            Has<GravityWell>,
        ),
        (With<LocalPlayer>, Without<Pilot>, Without<BuildMode>),
    >,
    mut evr_jump: EventReader<Jump>,
    mut q_camera: Query<(&GlobalTransform, &mut Transform, &mut CameraHelper), With<MainCamera>>,
    mut q_local_trans: Query<&mut Transform, (With<LocalPlayer>, Without<MainCamera>)>,
    q_main_cam_ent: Query<Entity, With<MainCamera>>,
    q_show_cursor: Query<(), With<ShowCursor>>,
    q_exerts_gravity: Query<(), With<Planet>>,
) {
    let any_open_menus = !q_show_cursor.is_empty();

    let Ok((cam_trans, mut cam_trans_local, mut cam_helper)) = q_camera.single_mut() else {
        return;
    };

    // This will be err if the player is piloting a ship
    let Ok((mut velocity, player_transform, player_alignment, grounded, under_gravity_well)) = q_local_player.single_mut() else {
        return;
    };

    if let Some(player_alignment) = player_alignment {
        let max_speed: f32 = if !any_open_menus && input_handler.check_pressed(CosmosInputs::Sprint) {
            20.0
        } else {
            3.0
        };

        let player_rot = Quat::from_affine3(&player_transform.affine());
        let player_inv_rot = player_rot.inverse();

        let mut forward = *cam_trans_local.forward();
        let mut right = *cam_trans_local.right();
        let up = Vec3::Y;

        forward.y = 0.0;
        right.y = 0.0;

        forward = forward.normalize_or_zero() * 100.0;
        right = right.normalize_or_zero() * 100.0;

        // TODO This is stupid - please rework this later.
        let normalize_y = !under_gravity_well && !q_exerts_gravity.contains(player_alignment.aligned_to);

        let movement_up = up.normalize_or_zero() * if normalize_y { 100.0 } else { 0.0 };

        let time = time.delta_secs();

        let mut new_linvel = player_inv_rot * velocity.linvel;

        // Simulate friction
        if grounded {
            new_linvel.x *= 0.5;
            new_linvel.z *= 0.5;
        }

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
            if evr_jump.read().next().is_some() && grounded {
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

        if !normalize_y {
            let y = new_linvel.y;

            new_linvel.y = 0.0;

            if new_linvel.dot(new_linvel) > max_speed * max_speed {
                new_linvel = new_linvel.normalize_or_zero() * max_speed;
            }

            new_linvel.y = y;
        } else if new_linvel.dot(new_linvel) > max_speed * max_speed {
            new_linvel = new_linvel.normalize_or_zero() * max_speed;
        }

        velocity.linvel = player_rot * new_linvel;
    } else {
        // let Ok(main_cam) = q_main_cam_ent.single() else {
        //     return;
        // };
        // let Some(cam_g_trans) = compute_totally_accurate_global_transform(main_cam, &q_trans) else {
        //     error!("Invalid heirarchy!");
        //     return;
        // };

        let accel = 3.0 * time.delta_secs();

        let cam_g_trans = cam_trans;

        let forward = cam_g_trans.forward() * accel;
        let up = cam_g_trans.up() * accel;
        let right = cam_g_trans.right() * accel;

        let max_speed = 7.0;

        let mut new_linvel = Vec3::ZERO;

        if input_handler.check_pressed(CosmosInputs::MoveForward) {
            new_linvel += forward;
        }
        if input_handler.check_pressed(CosmosInputs::MoveBackward) {
            new_linvel -= forward;
        }
        if input_handler.check_pressed(CosmosInputs::MoveUp) {
            new_linvel += up;
        }
        if input_handler.check_pressed(CosmosInputs::MoveDown) {
            new_linvel -= up;
        }
        if input_handler.check_pressed(CosmosInputs::MoveLeft) {
            new_linvel -= right;
        }
        if input_handler.check_pressed(CosmosInputs::MoveRight) {
            new_linvel += right;
        }

        new_linvel = new_linvel.normalize_or_zero() * accel;
        new_linvel += velocity.linvel;

        if input_handler.check_pressed(CosmosInputs::SlowDown) {
            let mut amt = new_linvel * 0.5;
            if amt.dot(amt) > max_speed * max_speed {
                amt = amt.normalize() * max_speed;
            }
            new_linvel -= amt;
        }

        velocity.linvel = new_linvel.clamp_length_max(max_speed);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Player movement inputs are handled and applied to their velocity
pub enum PlayerMovementSet {
    /// Player movement inputs are handled and applied to their velocity
    ProcessPlayerMovement,
}

#[derive(Event, Default)]
struct Jump;

fn jump_ev(inputs: InputChecker, mut evw_jump: EventWriter<Jump>) {
    if inputs.check_just_pressed(CosmosInputs::Jump) {
        evw_jump.write_default();
    }
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        FixedUpdate,
        PlayerMovementSet::ProcessPlayerMovement.before(LocationPhysicsSet::DoPhysics),
    );

    app.add_systems(
        FixedUpdate,
        (append_grounded_check, check_grounded)
            .run_if(in_state(GameState::Playing))
            .before(PlayerMovementSet::ProcessPlayerMovement)
            .chain(),
    );

    app.add_systems(Update, jump_ev);

    app.add_systems(
        FixedUpdate,
        process_player_movement
            .ambiguous_with(LaserSystemSet::SendHitEvents)
            .in_set(FixedUpdateSet::Main)
            .in_set(PlayerMovementSet::ProcessPlayerMovement)
            .run_if(in_state(GameState::Playing)),
    )
    .add_event::<Jump>();
}
