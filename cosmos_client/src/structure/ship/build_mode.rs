//! Handles the build mode logic on the client-side

use bevy::{
    prelude::{in_state, App, IntoSystemConfigs, Query, Res, ResMut, Transform, Update, Vec3, With, Without},
    time::Time,
};
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient},
    structure::{chunk::CHUNK_DIMENSIONSF, ship::build_mode::BuildMode},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    netty::flags::LocalPlayer,
    rendering::MainCamera,
    state::game_state::GameState,
    structure::planet::align_player::{self, PlayerAlignment},
};

fn exit_build_mode(
    input_handler: InputChecker,
    local_player_in_build_mode: Query<(), (With<LocalPlayer>, With<BuildMode>)>,
    mut client: ResMut<RenetClient>,
) {
    if local_player_in_build_mode.get_single().is_ok() {
        if input_handler.check_just_pressed(CosmosInputs::ToggleBuildMode) {
            client.send_message(
                NettyChannelClient::Reliable,
                cosmos_encoder::serialize(&ClientReliableMessages::ExitBuildMode),
            );
        }
    }
}

fn control_build_mode(
    input_handler: InputChecker,
    cam_query: Query<&Transform, With<MainCamera>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Velocity, Option<&PlayerAlignment>), (With<LocalPlayer>, With<BuildMode>, Without<MainCamera>)>,
) {
    if let Ok((mut transform, mut velocity, player_alignment)) = query.get_single_mut() {
        velocity.linvel = Vec3::ZERO;
        velocity.angvel = Vec3::ZERO;

        let cam_trans = transform.mul_transform(*cam_query.single());

        let max_speed: f32 = match input_handler.check_pressed(CosmosInputs::Sprint) {
            false => 5.0,
            true => 20.0,
        };

        let mut forward = cam_trans.forward();
        let mut right = cam_trans.right();
        let up = transform.up();

        match player_alignment.copied().unwrap_or_default().0 {
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

        forward = forward.normalize_or_zero() * max_speed;
        right = right.normalize_or_zero() * max_speed;
        let movement_up = up * max_speed;

        let time = time.delta_seconds();

        if input_handler.check_pressed(CosmosInputs::MoveForward) {
            transform.translation += forward * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveBackward) {
            transform.translation -= forward * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveUp) {
            transform.translation += movement_up * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveDown) {
            transform.translation -= movement_up * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveLeft) {
            transform.translation -= right * time;
        }
        if input_handler.check_pressed(CosmosInputs::MoveRight) {
            transform.translation += right * time;
        }

        let max = CHUNK_DIMENSIONSF * 10.0;

        transform.translation = transform.translation.clamp(Vec3::new(-max, -max, -max), Vec3::new(max, max, max));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (exit_build_mode, control_build_mode).chain().run_if(in_state(GameState::Playing)),
    );
}
