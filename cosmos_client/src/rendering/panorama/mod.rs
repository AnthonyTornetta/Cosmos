use std::{f32::consts::PI, time::Duration};

use bevy::{
    app::{App, Update},
    math::{Quat, Vec3},
    prelude::{not, resource_exists, Commands, Component, Entity, IntoSystemConfigs, Or, Query, Res, ResMut, Resource, With},
    render::view::{screenshot::ScreenshotManager, Visibility},
    time::{common_conditions::on_timer, Time},
    transform::components::Transform,
    ui::Node,
    window::PrimaryWindow,
};
use cosmos_core::ecs::NeedsDespawned;

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::{components::show_cursor::ShowCursor, item_renderer::RenderedItem},
};

use super::MainCamera;

const N_SCREENSHOTS: usize = 6;

#[derive(Resource)]
struct PanAngle(usize);

#[derive(Component)]
struct PanLocked;

#[derive(Component)]
struct OldVis(Visibility);

fn take_panorama(
    inputs: InputChecker,
    mut commands: Commands,
    mut q_camera: Query<&mut Transform, With<MainCamera>>,
    mut q_ui_elements: Query<(Entity, &mut Visibility), Or<(With<Node>, With<RenderedItem>)>>,
) {
    if inputs.check_just_pressed(CosmosInputs::PanoramaScreenshot) {
        commands.insert_resource(PanAngle(0));

        q_camera.single_mut().rotation = Quat::IDENTITY;
        commands.spawn((ShowCursor, PanLocked));

        for (entity, mut vis) in q_ui_elements.iter_mut() {
            commands.entity(entity).insert(OldVis(*vis));
            *vis = Visibility::Hidden;
        }
    }
}

fn taking_panorama(
    mut screenshot_manager: ResMut<ScreenshotManager>,
    mut pan_angle: ResMut<PanAngle>,
    mut commands: Commands,
    q_primary_window: Query<Entity, With<PrimaryWindow>>,
    mut q_camera: Query<&mut Transform, With<MainCamera>>,
) {
    let path = format!("./pan-screenshots/{}.png", pan_angle.0);

    screenshot_manager
        .save_screenshot_to_disk(q_primary_window.single(), path)
        .expect("Failed to save panorama screenshot!");

    pan_angle.0 += 1;
    q_camera.single_mut().rotation = Quat::from_axis_angle(Vec3::Y, pan_angle.0 as f32 / (N_SCREENSHOTS - 2) as f32 * PI * 2.0);

    if pan_angle.0 == N_SCREENSHOTS - 1 {
        q_camera.single_mut().rotation = Quat::from_axis_angle(Vec3::X, PI / 2.0);
    } else if pan_angle.0 == N_SCREENSHOTS {
        q_camera.single_mut().rotation = Quat::from_axis_angle(-Vec3::X, PI / 2.0);
        commands.remove_resource::<PanAngle>();
        commands.insert_resource(FinishedPan(0.0));
    }
}

fn restore_ui_after_panorama(
    mut commands: Commands,
    q_pan_locked: Query<Entity, With<PanLocked>>,
    mut q_ui_elements: Query<(Entity, &mut Visibility, &OldVis), Or<(With<Node>, With<RenderedItem>)>>,
    mut fin: ResMut<FinishedPan>,
    time: Res<Time>,
) {
    fin.0 += time.delta_seconds();

    if fin.0 < 0.2 {
        return;
    }

    commands.remove_resource::<FinishedPan>();

    for e in q_pan_locked.iter() {
        commands.entity(e).insert(NeedsDespawned);
    }

    for (entity, mut vis, old_viz) in q_ui_elements.iter_mut() {
        *vis = old_viz.0;
        commands.entity(entity).remove::<OldVis>();
    }
}

#[derive(Resource)]
struct FinishedPan(f32);

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            taking_panorama
                .run_if(resource_exists::<PanAngle>)
                .run_if(on_timer(Duration::from_millis(100))),
            (take_panorama, restore_ui_after_panorama.run_if(resource_exists::<FinishedPan>))
                .chain()
                .run_if(not(resource_exists::<PanAngle>)),
        )
            .chain(),
    );
}
