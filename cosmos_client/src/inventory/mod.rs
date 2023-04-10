use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::{camera::ScalingMode, view::RenderLayers},
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

fn ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            projection: Projection::Orthographic(OrthographicProjection {
                scale: 3.0,
                scaling_mode: ScalingMode::FixedVertical(2.0),
                ..default()
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
        RenderLayers::from_layers(&[INVENTORY_SLOT_LAYER]),
    ));
}

struct Inventory3dModel {}

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

    let size = 0.2;

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
            .unwrap_or_else(|| {
                block_textures
                    .from_id("missing")
                    .expect("Missing texture should exist.")
            });

        let mult = size * 2.0;
        let sx = -(amt as f32) / 2.0 * mult + mult * i as f32;

        let mut transform = Transform::from_xyz(sx, 0.0, 0.0); //.looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);

        // These make it look cool
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
                        material: atlas.material.clone(),
                        transform,
                        ..default()
                    },
                    RenderLayers::from_layers(&[INVENTORY_SLOT_LAYER]),
                ))
                .id(),
        );
    }

    commands.spawn((
        DirectionalLightBundle {
            transform: Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            directional_light: DirectionalLight {
                ..Default::default()
            },
            ..Default::default()
        },
        RenderLayers::from_layers(&[INVENTORY_SLOT_LAYER]),
    ));

    let mut hotbar = commands.spawn(PbrBundle {
        transform: Transform::from_xyz(0.0, -2.7, 0.0),
        ..Default::default()
    });

    for child in children {
        hotbar.add_child(child);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(render_hotbar.in_schedule(OnEnter(GameState::Playing)))
        .add_startup_system(ui_camera);
}
