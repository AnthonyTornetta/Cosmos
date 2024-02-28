use bevy::{
    app::{App, Update},
    ecs::{
        component::Component,
        entity::Entity,
        query::{Added, Changed, Has, With, Without},
        removal_detection::RemovedComponents,
        schedule::{common_conditions::in_state, IntoSystemConfigs},
        system::{Commands, Query},
    },
    math::Vec3,
    reflect::Reflect,
    transform::components::Transform,
};
use cosmos_core::structure::{
    ship::{pilot::Pilot, Ship},
    systems::{camera_system::CameraSystem, StructureSystem, Systems},
    Structure,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    netty::flags::LocalPlayer,
    rendering::{CameraPlayerOffset, MainCamera},
    state::game_state::GameState,
};

use super::sync::sync_system;

#[derive(Debug, Component, Reflect, Clone, Copy)]
/// Which camera the client would prefer to look through
enum SelectedCamera {
    Camera(usize),
    ShipCore,
}

fn on_add_camera_system(q_select_camera: Query<Entity, (With<Ship>, Without<SelectedCamera>)>, mut commands: Commands) {
    for ent in &q_select_camera {
        commands.entity(ent).insert(SelectedCamera::ShipCore);
    }
}

fn swap_camera(
    inputs: InputChecker,
    q_pilot: Query<&Pilot, With<LocalPlayer>>,
    q_camera_system: Query<&CameraSystem>,
    mut q_ship_query: Query<(&mut SelectedCamera, &Systems)>,
) {
    let Ok(pilot) = q_pilot.get_single() else {
        return;
    };

    let Ok((mut selected_camera, systems)) = q_ship_query.get_mut(pilot.entity) else {
        return;
    };

    let Ok(cam_system) = systems.query(&q_camera_system) else {
        return;
    };

    if inputs.check_just_pressed(CosmosInputs::SwapCameraLeft) {
        *selected_camera = match *selected_camera {
            SelectedCamera::Camera(idx) => {
                if idx == 0 {
                    SelectedCamera::ShipCore
                } else {
                    SelectedCamera::Camera(idx - 1)
                }
            }
            SelectedCamera::ShipCore => {
                let locs = cam_system.camera_locations();
                if locs.is_empty() {
                    SelectedCamera::ShipCore
                } else {
                    SelectedCamera::Camera(cam_system.camera_locations().len() - 1)
                }
            }
        }
    }

    if inputs.check_just_pressed(CosmosInputs::SwapCameraRight) {
        *selected_camera = match *selected_camera {
            SelectedCamera::Camera(idx) => {
                if idx >= cam_system.camera_locations().len() - 1 {
                    SelectedCamera::ShipCore
                } else {
                    SelectedCamera::Camera(idx + 1)
                }
            }
            SelectedCamera::ShipCore => {
                if cam_system.camera_locations().is_empty() {
                    SelectedCamera::ShipCore
                } else {
                    SelectedCamera::Camera(0)
                }
            }
        }
    }

    match *selected_camera {
        SelectedCamera::Camera(idx) => {
            let len = cam_system.camera_locations().len();
            if idx > len {
                if len == 0 {
                    *selected_camera = SelectedCamera::ShipCore;
                } else {
                    *selected_camera = SelectedCamera::Camera(len - 1)
                }
            }
        }
        _ => {}
    }
}

fn on_change_selected_camera(
    mut main_camera: Query<&mut Transform, With<MainCamera>>,
    q_became_pilot: Query<Has<LocalPlayer>, Added<Pilot>>,
    q_pilot: Query<(&Pilot, &CameraPlayerOffset), With<LocalPlayer>>,
    q_selected_camera: Query<(Entity, &SelectedCamera, &Systems, &Structure)>,
    q_changed_stuff: Query<(Entity, &SelectedCamera, &Systems, &Structure), Changed<SelectedCamera>>,
    q_changed_camera_system: Query<(&StructureSystem, &CameraSystem), Changed<CameraSystem>>,
    q_camera_system: Query<&CameraSystem>,
) {
    let Ok((pilot, camera_player_offset)) = q_pilot.get_single() else {
        return;
    };
    let Ok(mut main_cam_trans) = main_camera.get_single_mut() else {
        return;
    };

    if !q_became_pilot.is_empty() {
        let Ok((_, selected_camera, systems, structure)) = q_selected_camera.get(pilot.entity) else {
            return;
        };

        let Ok(camera_system) = systems.query(&q_camera_system) else {
            return;
        };

        adjust_camera(camera_system, selected_camera, structure, &mut main_cam_trans, camera_player_offset);
    }

    for (ent, selected_camera, systems, structure) in q_changed_stuff.iter() {
        if pilot.entity != ent {
            continue;
        }

        let Ok(camera_system) = systems.query(&q_camera_system) else {
            continue;
        };

        adjust_camera(camera_system, selected_camera, structure, &mut main_cam_trans, camera_player_offset);
    }

    for (ss, camera_system) in q_changed_camera_system.iter() {
        let Ok((ent, selected_camera, _, structure)) = q_selected_camera.get(ss.structure_entity()) else {
            continue;
        };

        if pilot.entity != ent {
            continue;
        }

        adjust_camera(camera_system, selected_camera, structure, &mut main_cam_trans, camera_player_offset);
    }
}

fn adjust_camera(
    camera_system: &CameraSystem,
    selected_camera: &SelectedCamera,
    structure: &Structure,
    main_cam_trans: &mut Transform,
    cam_offset: &CameraPlayerOffset,
) {
    let cams = camera_system.camera_locations();
    let cam_block_coords = match *selected_camera {
        SelectedCamera::Camera(idx) => {
            let Some(cam) = cams.get(idx) else {
                return;
            };
            *cam
        }
        SelectedCamera::ShipCore => Ship::ship_core_block_coords(structure),
    };

    let local_pos = structure.block_relative_position(cam_block_coords);
    let offset = cam_offset.0 - Vec3::splat(0.5);
    main_cam_trans.translation = local_pos + offset;
}

fn on_stop_piloting(
    mut q_removed_pilots: RemovedComponents<Pilot>,
    q_player: Query<&CameraPlayerOffset, With<LocalPlayer>>,
    mut q_main_camera: Query<&mut Transform, With<MainCamera>>,
) {
    for ent in q_removed_pilots.read() {
        let Ok(cam_offset) = q_player.get(ent) else {
            continue;
        };

        let Ok(mut trans) = q_main_camera.get_single_mut() else {
            return;
        };

        trans.translation = cam_offset.0;
    }
}

pub(super) fn register(app: &mut App) {
    sync_system::<CameraSystem>(app);

    app.add_systems(
        Update,
        (on_add_camera_system, swap_camera, on_change_selected_camera, on_stop_piloting)
            .chain()
            .run_if(in_state(GameState::Playing)),
    )
    .register_type::<SelectedCamera>();
}
