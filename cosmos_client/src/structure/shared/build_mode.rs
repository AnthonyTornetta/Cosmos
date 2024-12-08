//! Handles the build mode logic on the client-side

use bevy::{
    color::LinearRgba,
    math::primitives::Cuboid,
    pbr::{MeshMaterial3d, NotShadowCaster, NotShadowReceiver},
    prelude::{
        in_state, App, AssetServer, Assets, BuildChildren, Changed, ChildBuild, Commands, Component, DespawnRecursiveExt, Entity,
        EventReader, IntoSystemConfigs, Mesh, Mesh3d, Name, Parent, Query, Res, ResMut, Transform, Update, Vec3, With, Without,
    },
    time::Time,
};
use bevy_rapier3d::prelude::Velocity;
use bevy_renet2::renet2::RenetClient;
use cosmos_core::{
    block::block_events::BlockEventsSet,
    netty::{client::LocalPlayer, client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient},
    state::GameState,
    structure::{
        chunk::CHUNK_DIMENSIONSF,
        coordinates::BlockCoordinate,
        shared::{
            build_mode::{BuildAxis, BuildMode, BuildModeSet, ExitBuildModeEvent},
            DespawnWithStructure,
        },
        Structure,
    },
};

use crate::{
    asset::repeating_material::{Repeats, UnlitRepeatedMaterial},
    entities::player::player_movement::{process_player_movement, PlayerMovementSet},
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    interactions::block_interactions::LookingAt,
    rendering::MainCamera,
    structure::planet::align_player::{self, PlayerAlignment},
    ui::components::show_cursor::no_open_menus,
};

fn exit_build_mode(
    input_handler: InputChecker,
    local_player_in_build_mode: Query<(), (With<LocalPlayer>, With<BuildMode>)>,
    mut client: ResMut<RenetClient>,
) {
    if local_player_in_build_mode.get_single().is_ok() && input_handler.check_just_pressed(CosmosInputs::ToggleBuildMode) {
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::ExitBuildMode),
        );
    }
}

#[derive(Component, Clone, Copy, Default)]
struct SymmetryVisuals(Option<Entity>, Option<Entity>, Option<Entity>);

fn control_build_mode(
    input_handler: InputChecker,
    cam_query: Query<&Transform, With<MainCamera>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Velocity, Option<&PlayerAlignment>), (With<LocalPlayer>, With<BuildMode>, Without<MainCamera>)>,
) {
    let Ok((mut transform, mut velocity, player_alignment)) = query.get_single_mut() else {
        return;
    };
    velocity.linvel = Vec3::ZERO;
    velocity.angvel = Vec3::ZERO;

    let cam_trans = transform.mul_transform(*cam_query.single());

    let max_speed: f32 = match input_handler.check_pressed(CosmosInputs::Sprint) {
        false => 5.0,
        true => 20.0,
    };

    let mut forward = *cam_trans.forward();
    let mut right = *cam_trans.right();
    let up = *transform.up();

    match player_alignment.map(|x| x.axis).unwrap_or_default() {
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

    let time = time.delta_secs();

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

fn place_symmetries(
    mut client: ResMut<RenetClient>,
    input_handler: InputChecker,
    query: Query<&LookingAt, (With<LocalPlayer>, With<BuildMode>)>,
) {
    let Ok(looking_at) = query.get_single() else {
        return;
    };

    let clearing = input_handler.check_pressed(CosmosInputs::ClearSymmetry);

    let looking_at_block = if !clearing {
        looking_at.looking_at_any.map(|x| x.block)
    } else {
        None
    };

    if !clearing && looking_at_block.is_none() {
        return;
    }

    if input_handler.check_just_pressed(CosmosInputs::SymmetryX) {
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::SetSymmetry {
                axis: BuildAxis::X,
                coordinate: looking_at_block.map(|block| block.x()),
            }),
        )
    }

    if input_handler.check_just_pressed(CosmosInputs::SymmetryY) {
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::SetSymmetry {
                axis: BuildAxis::Y,
                coordinate: looking_at_block.map(|block| block.y()),
            }),
        )
    }

    if input_handler.check_just_pressed(CosmosInputs::SymmetryZ) {
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::SetSymmetry {
                axis: BuildAxis::Z,
                coordinate: looking_at_block.map(|block| block.z()),
            }),
        )
    }
}

fn clear_visuals(
    parent_query: Query<&Parent>,
    visuals_query: Query<&SymmetryVisuals>,
    mut event_reader: EventReader<ExitBuildModeEvent>,
    mut commands: Commands,
) {
    for ev in event_reader.read() {
        let Ok(parent) = parent_query.get(ev.player_entity).map(|p| p.get()) else {
            continue;
        };
        let Some(mut ecmds) = commands.get_entity(parent) else {
            continue;
        };

        ecmds.remove::<SymmetryVisuals>();

        if let Ok(sym_visuals) = visuals_query.get(parent) {
            if let Some(ent) = sym_visuals.0 {
                commands.entity(ent).despawn_recursive();
            }
            if let Some(ent) = sym_visuals.1 {
                commands.entity(ent).despawn_recursive();
            }
            if let Some(ent) = sym_visuals.2 {
                commands.entity(ent).despawn_recursive();
            }
        }
    }
}

fn change_visuals(
    mut commands: Commands,
    query: Query<(&BuildMode, &Parent), (With<LocalPlayer>, Changed<BuildMode>)>,
    structure_query: Query<&Structure>,
    visuals: Query<&SymmetryVisuals>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<UnlitRepeatedMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let Ok((build_mode, parent)) = query.get_single() else {
        return;
    };
    let structure_entity = parent.get();
    let Ok(structure) = structure_query.get(structure_entity) else {
        return;
    };

    let mut visuals = visuals.get(structure_entity).copied().unwrap_or_default();

    if let Some(ent) = visuals.0 {
        commands.entity(ent).despawn_recursive();
        visuals.0 = None;
    }
    if let Some(ent) = visuals.1 {
        commands.entity(ent).despawn_recursive();
        visuals.1 = None;
    }
    if let Some(ent) = visuals.2 {
        commands.entity(ent).despawn_recursive();
        visuals.2 = None;
    }

    let texture_handle = asset_server.load("cosmos/images/misc/symmetry.png");

    let size = structure.block_dimensions().x;

    if let Some(coords) = build_mode.get_symmetry(BuildAxis::X) {
        let coords = structure.block_relative_position(BlockCoordinate::new(coords, 0, 0));

        commands.entity(structure_entity).with_children(|ecmds| {
            visuals.0 = Some(
                ecmds
                    .spawn((
                        DespawnWithStructure,
                        NotShadowCaster,
                        NotShadowReceiver,
                        Name::new("X Axis - build mode"),
                        Mesh3d(meshes.add(Cuboid::new(0.001, size as f32, size as f32))),
                        MeshMaterial3d(materials.add(UnlitRepeatedMaterial {
                            repeats: Repeats {
                                horizontal: size as u32 / 4,
                                vertical: size as u32 / 4,
                                ..Default::default()
                            },
                            texture: texture_handle.clone(),
                            color: LinearRgba {
                                red: 1.0,
                                green: 0.0,
                                blue: 0.0,
                                alpha: 1.0,
                            },
                        })),
                        Transform::from_xyz(coords.x, 0.5, 0.5),
                    ))
                    .id(),
            );
        });
    }

    if let Some(coords) = build_mode.get_symmetry(BuildAxis::Y) {
        let coords = structure.block_relative_position(BlockCoordinate::new(0, coords, 0));

        commands.entity(structure_entity).with_children(|ecmds| {
            visuals.1 = Some(
                ecmds
                    .spawn((
                        DespawnWithStructure,
                        NotShadowCaster,
                        NotShadowReceiver,
                        Name::new("Y Axis - build mode"),
                        Mesh3d(meshes.add(Cuboid::new(size as f32, 0.001, size as f32))),
                        MeshMaterial3d(materials.add(UnlitRepeatedMaterial {
                            repeats: Repeats {
                                horizontal: size as u32 / 4,
                                vertical: size as u32 / 4,
                                ..Default::default()
                            },
                            texture: texture_handle.clone(),
                            color: LinearRgba {
                                red: 0.0,
                                green: 1.0,
                                blue: 0.0,
                                alpha: 1.0,
                            },
                        })),
                        Transform::from_xyz(0.5, coords.y, 0.5),
                    ))
                    .id(),
            );
        });
    }

    if let Some(coords) = build_mode.get_symmetry(BuildAxis::Z) {
        let coords = structure.block_relative_position(BlockCoordinate::new(0, 0, coords));

        commands.entity(structure_entity).with_children(|ecmds| {
            visuals.2 = Some(
                ecmds
                    .spawn((
                        DespawnWithStructure,
                        NotShadowCaster,
                        NotShadowReceiver,
                        Name::new("Z Axis - build mode"),
                        Mesh3d(meshes.add(Cuboid::new(size as f32, size as f32, 0.001))),
                        MeshMaterial3d(materials.add(UnlitRepeatedMaterial {
                            repeats: Repeats {
                                horizontal: size as u32 / 4,
                                vertical: size as u32 / 4,
                                ..Default::default()
                            },
                            texture: texture_handle.clone(),
                            color: LinearRgba {
                                red: 0.0,
                                green: 0.0,
                                blue: 1.0,
                                alpha: 1.0,
                            },
                        })),
                        Transform::from_xyz(0.5, 0.5, coords.z),
                    ))
                    .id(),
            );
        });
    }

    commands.entity(structure_entity).insert(visuals);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            (
                place_symmetries,
                exit_build_mode,
                control_build_mode
                    .in_set(PlayerMovementSet::ProcessPlayerMovement)
                    .ambiguous_with(process_player_movement), // this system will run if process_player_movement doesn't
            )
                .chain()
                .in_set(BlockEventsSet::ProcessEvents)
                .run_if(no_open_menus),
            change_visuals,
            clear_visuals.after(BuildModeSet::ExitBuildMode),
        )
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
