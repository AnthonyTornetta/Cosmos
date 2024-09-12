//! Handles the basic player movement while walking around. This is not responsible for piloting ships. See [`ship_movement`] for that.

use bevy::prelude::*;
use bevy_rapier3d::{
    plugin::{RapierContextAccess, RapierContextEntityLink},
    prelude::{ActiveEvents, Collider, Sensor, Velocity},
};
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
    structure::planet::align_player::PlayerAlignment,
    ui::components::show_cursor::ShowCursor,
};

#[derive(Component, Debug)]
/// Indicates if the player is touching the ground
pub struct Grounded;

#[derive(Component)]
struct GroundedChecker;

fn append_grounded_check(mut commands: Commands, q_player: Query<Entity, Added<LocalPlayer>>) {
    let Ok(player_ent) = q_player.get_single() else {
        return;
    };
    commands.entity(player_ent).with_children(|p| {
        p.spawn((
            GroundedChecker,
            SpatialBundle {
                transform: Transform::from_xyz(0.0, -0.80, 0.0),
                ..Default::default()
            },
            Name::new("Ground checker"),
            Collider::cuboid(0.1, 0.1, 0.1),
            Sensor,
            ActiveEvents::COLLISION_EVENTS,
        ));
    });
}

fn check_grounded(
    mut commands: Commands,
    rapier_context: RapierContextAccess,
    q_player: Query<Entity, With<LocalPlayer>>,
    q_ground_checker: Query<(&RapierContextEntityLink, Entity), With<GroundedChecker>>,
) {
    let Ok((rapier_link, collider_ent)) = q_ground_checker.get_single() else {
        return;
    };

    let Ok(player_ent) = q_player.get_single() else {
        return;
    };

    let context = rapier_context.context(rapier_link);

    let touching_ground = context.intersection_pairs_with(collider_ent).any(|x| x.2);
    if touching_ground {
        commands.entity(player_ent).insert(Grounded);
    } else {
        commands.entity(player_ent).remove::<Grounded>();
    }
}

pub(crate) fn process_player_movement(
    time: Res<Time>,
    input_handler: InputChecker,
    mut q_local_player: Query<
        (&mut Velocity, &GlobalTransform, Option<&PlayerAlignment>, Option<&Grounded>),
        (With<LocalPlayer>, Without<Pilot>, Without<BuildMode>),
    >,
    q_camera: Query<&Transform, With<MainCamera>>,
    q_show_cursor: Query<(), With<ShowCursor>>,
) {
    let any_open_menus = !q_show_cursor.is_empty();

    let Ok(cam_trans) = q_camera.get_single() else {
        return;
    };

    // This will be err if the player is piloting a ship
    let Ok((mut velocity, player_transform, player_alignment, grounded)) = q_local_player.get_single_mut() else {
        return;
    };

    let max_speed: f32 = if !any_open_menus && input_handler.check_pressed(CosmosInputs::Sprint) {
        20.0
    } else {
        3.0
    };

    let player_rot = Quat::from_affine3(&player_transform.affine());
    let player_inv_rot = player_rot.inverse();

    let mut forward = *cam_trans.forward();
    let mut right = *cam_trans.right();
    let up = Vec3::Y;

    forward.y = 0.0;
    right.y = 0.0;

    forward = forward.normalize_or_zero() * 100.0;
    right = right.normalize_or_zero() * 100.0;
    let movement_up = up.normalize_or_zero() * 2.0;

    let time = time.delta_seconds();

    let mut new_linvel = player_inv_rot * velocity.linvel;

    // Simulate friction
    if grounded.is_some() {
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

    velocity.linvel = player_rot * new_linvel;
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
        (append_grounded_check, check_grounded).run_if(in_state(GameState::Playing)).chain(),
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
