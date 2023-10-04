//! Renders items as 3d models at based off the RenderItem present in a UI element

use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::{camera::ScalingMode, view::RenderLayers},
    window::PrimaryWindow,
};
use cosmos_core::{
    block::{Block, BlockFace},
    blockitems::BlockItems,
    ecs::NeedsDespawned,
    item::Item,
    registry::{identifiable::Identifiable, many_to_one::ManyToOneRegistry, Registry},
};

use crate::{
    asset::asset_loading::{BlockTextureIndex, MaterialDefinition},
    rendering::{BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder},
};

const INVENTORY_SLOT_LAYER: u8 = 0b1;

#[derive(Component)]
struct UICamera;

fn create_ui_camera(mut commands: Commands) {
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

#[derive(Debug, Component)]
/// Put this onto a UI element to render a 3D item there
pub struct RenderItem {
    /// The item's id
    pub item_id: u16,
}

#[derive(Debug, Component)]
struct RenderedItem {
    /// Points to the UI entity that had the `RenderItem` that created this
    ui_element_entity: Entity,
    item_id: u16,
    based_off: Vec3,
}

fn render_items(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,

    block_items: Res<BlockItems>,
    items: Res<Registry<Item>>,
    blocks: Res<Registry<Block>>,

    materials_registry: Res<ManyToOneRegistry<Block, MaterialDefinition>>,
    block_textures: Res<Registry<BlockTextureIndex>>,
    block_meshes: Res<BlockMeshRegistry>,

    mut removed_render_items: RemovedComponents<RenderItem>,
    changed_render_items: Query<(Entity, &RenderItem, &GlobalTransform), Or<(Changed<RenderItem>, Changed<GlobalTransform>)>>,
    rendered_items: Query<(Entity, &RenderedItem)>,
) {
    for entity in removed_render_items.iter() {
        if let Some((rendered_item_entity, _)) = rendered_items
            .iter()
            .find(|(_, rendered_item)| rendered_item.ui_element_entity == entity)
        {
            if let Some(mut ecmds) = commands.get_entity(rendered_item_entity) {
                ecmds.insert(NeedsDespawned);
            }
        }
    }

    for (entity, changed_render_item, transform) in changed_render_items.iter() {
        let size = 0.8;
        let translation = transform.translation();

        let to_create = if let Some((rendered_item_entity, rendered_item)) = rendered_items
            .iter()
            .find(|(_, rendered_item)| rendered_item.ui_element_entity == entity)
        {
            if rendered_item.item_id == changed_render_item.item_id {
                // We're already displaying that item, no need to recalculate everything
                continue;
            }

            rendered_item_entity
        } else {
            let mut transform = Transform::from_rotation(Quat::from_xyzw(0.18354653, 0.37505528, 0.07602747, 0.90546346)); // This makes it look cool

            // hide it till we position it properly
            transform.translation.x = -1000000.0;

            commands
                .spawn(PbrBundle {
                    transform,
                    ..Default::default()
                })
                .id()
        };

        let item = items.from_numeric_id(changed_render_item.item_id);

        let Some(block_id) = block_items.block_from_item(item) else {
            continue;
        };

        let block = blocks.from_numeric_id(block_id);

        let index = block_textures
            .from_id(block.unlocalized_name())
            .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

        let Some(block_mesh_info) = block_meshes.get_value(block) else {
            continue;
        };

        let mut mesh_builder = CosmosMeshBuilder::default();

        let Some(material) = materials_registry.get_value(block) else {
            warn!("Missing material for block {}", block.unlocalized_name());
            continue;
        };

        if block_mesh_info.has_multiple_face_meshes() {
            for face in [BlockFace::Top, BlockFace::Left, BlockFace::Back] {
                let Some(mut mesh_info) = block_mesh_info.info_for_face(face).cloned() else {
                    break;
                };

                mesh_info.scale(Vec3::new(size, size, size));

                let Some(image_index) = index.atlas_index_from_face(face) else {
                    continue;
                };

                let uvs = material.uvs_for_index(image_index);

                mesh_builder.add_mesh_information(&mesh_info, Vec3::ZERO, uvs);
            }
        } else {
            let Some(mut mesh_info) = block_mesh_info.info_for_whole_block().cloned() else {
                break;
            };

            mesh_info.scale(Vec3::new(size, size, size));

            let Some(image_index) = index.atlas_index("all") else {
                continue;
            };

            let uvs = material.uvs_for_index(image_index);

            mesh_builder.add_mesh_information(&mesh_info, Vec3::ZERO, uvs);
        }

        commands.entity(to_create).insert((
            RenderedItem {
                based_off: translation,
                ui_element_entity: entity,
                item_id: changed_render_item.item_id,
            },
            meshes.add(mesh_builder.build_mesh()),
            material.unlit_material().clone(),
            RenderLayers::from_layers(&[INVENTORY_SLOT_LAYER]),
            Name::new("Rendered Inventory Item"),
        ));
    }
}

fn update_rendered_items_transforms(
    query: Query<(Entity, &GlobalTransform), (With<RenderItem>, Changed<GlobalTransform>)>,
    mut rendered_items: Query<&mut RenderedItem>,
) {
    for (entity, changed_transform) in query.iter() {
        if let Some(mut rendered_item) = rendered_items.iter_mut().find(|x| x.ui_element_entity == entity) {
            rendered_item.based_off = changed_transform.translation();
        }
    }
}

fn reposition_ui_items(query: Query<&Window, With<PrimaryWindow>>, mut rendered_items: Query<(&mut Transform, &RenderedItem)>) {
    let Ok(window) = query.get_single() else {
        return;
    };

    for (mut transform, rendered_item) in rendered_items.iter_mut() {
        let translation = rendered_item.based_off;

        let (mut x, mut y) = (translation.x, translation.y);

        let (w, h) = (window.width(), window.height());

        // normalizes x/y to be [-1, 1]
        (x, y) = ((x / w - 0.5) * 2.0, (y / h - 0.5) * 2.0);

        // magic equations derived from trial + error to reposition stuff based on window size
        let x_num = 0.0124979 * w - 0.016775;
        let y_num = 0.0124566 * h + 0.10521;

        x *= x_num;
        y *= -y_num;

        transform.translation.x = x;
        transform.translation.y = y;
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (update_rendered_items_transforms, reposition_ui_items, render_items).chain(),
    )
    .add_systems(Startup, create_ui_camera);
}