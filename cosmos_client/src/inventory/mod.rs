//! This whole module is pretty bad because it relies on a whole lot of magic numbers.
//!
//! This will need to be redone once more slots are added.

use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::{camera::ScalingMode, view::RenderLayers},
    window::PrimaryWindow,
};
use cosmos_core::{
    block::{Block, BlockFace},
    blockitems::BlockItems,
    inventory::Inventory,
    item::Item,
    registry::{identifiable::Identifiable, Registry},
};

use crate::{
    asset::asset_loading::{BlockTextureIndex, MainAtlas},
    netty::flags::LocalPlayer,
    rendering::{BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder},
    state::game_state::GameState,
};

const INVENTORY_SLOT_LAYER: u8 = 10;

#[derive(Component)]
struct UICamera;

fn ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            projection: Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::WindowSize(40.0),
                ..Default::default()
            }),
            camera_3d: Camera3d {
                clear_color: ClearColorConfig::None,
                ..Default::default()
            },
            camera: Camera {
                order: 1,
                hdr: true, // this has to be true or the camera doesn't render over the main one correctly.
                ..Default::default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        UICamera,
        RenderLayers::from_layers(&[INVENTORY_SLOT_LAYER]),
    ));
}

fn render_hotbar(
    inventory: Query<&Inventory, With<LocalPlayer>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,

    block_items: Res<BlockItems>,
    items: Res<Registry<Item>>,
    blocks: Res<Registry<Block>>,

    atlas: Res<MainAtlas>,
    block_textures: Res<Registry<BlockTextureIndex>>,
    block_meshes: Res<BlockMeshRegistry>,
) {
    let Ok(inventory) = inventory.get_single() else {
        return;
    };

    let amt = 9;

    let size = 0.8;

    let mut children = vec![];

    for (i, item) in inventory.iter().take(amt).enumerate() {
        let Some(item_stack) = item else {
            continue;
        };

        let item = items.from_numeric_id(item_stack.item_id());

        let Some(block_id) = block_items.block_from_item(item) else {
            continue;
        };

        let block = blocks.from_numeric_id(block_id);

        let index = block_textures
            .from_id(block.unlocalized_name())
            .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

        let multiplier = size * 2.0;
        let slot_x = -(amt as f32) / 2.0 * multiplier + multiplier * (i as f32 + 0.5);

        let mut transform = Transform::from_xyz(slot_x, 0.0, 0.0);

        // This makes it look cool
        transform.rotation = Quat::from_xyzw(0.18354653, 0.37505528, 0.07602747, 0.90546346);

        let Some(block_mesh_info) = block_meshes.get_value(block) else {
            continue;
        };

        let mut mesh_builder = CosmosMeshBuilder::default();

        for face in [BlockFace::Top, BlockFace::Left, BlockFace::Back] {
            let mut mesh_info = block_mesh_info.info_for_face(face).clone();
            mesh_info.scale(Vec3::new(size, size, size));

            let Some(image_index) = index.atlas_index_from_face(face) else {
                continue;
            };

            let uvs = atlas.uvs_for_index(image_index);

            mesh_builder.add_mesh_information(&mesh_info, Vec3::ZERO, uvs);
        }

        children.push(
            commands
                .spawn((
                    PbrBundle {
                        mesh: meshes.add(mesh_builder.build_mesh()),
                        material: atlas.unlit_material.clone(),
                        transform,
                        ..default()
                    },
                    RenderLayers::from_layers(&[INVENTORY_SLOT_LAYER]),
                ))
                .id(),
        );
    }

    let mut hotbar = commands.spawn((
        PbrBundle {
            transform: Transform::from_xyz(0.0, -8.22, 0.0),
            ..Default::default()
        },
        HotbarLocation,
    ));

    for child in children {
        hotbar.add_child(child);
    }
}

#[derive(Component)]
struct HotbarLocation;

pub(super) fn register(app: &mut App) {
    app.add_system(
        |query: Query<&Window, With<PrimaryWindow>>, mut cam: Query<&mut Transform, With<HotbarLocation>>| {
            let Ok(window) = query.get_single() else {
                return;
            };
            let Ok(mut transform) = cam.get_single_mut() else {
                return;
            };

            transform.translation.y = -0.012533 * window.height() + 0.804;
        },
    )
    .add_system(render_hotbar.in_schedule(OnEnter(GameState::Playing)))
    .add_startup_system(ui_camera);
}
