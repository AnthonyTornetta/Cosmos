use std::f32::consts::PI;

use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::{
        camera::{CameraOutputMode, RenderTarget, ScalingMode},
        view::RenderLayers,
    },
};
use cosmos_core::{block::Block, inventory::Inventory, registry::Registry};

use crate::{netty::flags::LocalPlayer, rendering::BlockMeshRegistry};

const INVENTORY_SLOT_LAYER: u8 = 10;

fn ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            projection: Projection::Orthographic(OrthographicProjection {
                scale: 3.0,
                scaling_mode: ScalingMode::FixedVertical(2.0),
                ..default()
            }),
            camera_3d: Camera3d {
                // clear_color: ClearColorConfig::None,
                ..Default::default()
            },
            camera: Camera {
                order: 1,
                ..Default::default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        RenderLayers::from_layers(&[INVENTORY_SLOT_LAYER]),
    ));
}

struct Inventory3dModel {}

fn render_hotbar(
    // inventory: Query<&Inventory, With<LocalPlayer>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
) {
    let amt = 20;

    for i in 0..amt {
        let mult = 2.0;
        let sx = -(amt as f32) / 2.0 * mult + mult * i as f32;

        let mut transform = Transform::from_xyz(sx, 0.0, -5.0); //.looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);

        let looking_vec = Vec3::new(1.0, -0.3, 1.0).normalize();

        transform.look_at(
            Vec3::new(
                sx + looking_vec.x,
                0.0 + looking_vec.y,
                -5.0 + looking_vec.z,
            ),
            Vec3::Y,
        );

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                material: materials.add(StandardMaterial {
                    base_color_texture: Some(server.load("images/blocks/dirt.png")),
                    // unlit: true,
                    ..Default::default()
                }),
                transform,
                ..default()
            },
            RenderLayers::from_layers(&[INVENTORY_SLOT_LAYER]),
        ));
    }

    commands.spawn((
        DirectionalLightBundle {
            transform: Transform::from_xyz(0.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            directional_light: DirectionalLight {
                ..Default::default()
            },
            ..Default::default()
        },
        RenderLayers::from_layers(&[INVENTORY_SLOT_LAYER]),
    ));
}

pub(super) fn register(app: &mut App) {
    app.add_startup_system(render_hotbar)
        .add_startup_system(ui_camera);
}
