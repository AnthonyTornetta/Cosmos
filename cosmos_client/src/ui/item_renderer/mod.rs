//! Renders items as 3d models at based off the RenderItem present in a UI element

use std::f32::consts::PI;

use bevy::{
    prelude::*,
    render::{camera::ScalingMode, view::RenderLayers},
    window::PrimaryWindow,
};
use cosmos_core::{
    block::{block_face::BlockFace, Block},
    blockitems::BlockItems,
    ecs::NeedsDespawned,
    item::Item,
    registry::{identifiable::Identifiable, many_to_one::ManyToOneRegistry, Registry},
};

use crate::{
    asset::{
        asset_loading::{BlockNeighbors, BlockTextureIndex, CosmosTextureAtlas, ItemTextureIndex},
        materials::{
            AddMaterialEvent, BlockMaterialMapping, ItemMaterialMapping, MaterialDefinition, MaterialType, MaterialsSystemSet,
            RemoveAllMaterialsEvent,
        },
        texture_atlas::SquareTextureAtlas,
    },
    item::item_mesh::create_item_mesh,
    rendering::{BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder},
    state::game_state::GameState,
};

use super::{UiMiddleRoot, UiSystemSet, UiTopRoot};

const INVENTORY_SLOT_LAYER: usize = 0b01;
const MIDDLE_INVENTORY_SLOT_LAYER: usize = 0b10;

pub(crate) fn create_ui_cameras(mut commands: Commands) {
    commands.spawn((
        Name::new("UI Top Camera"),
        UiTopRoot,
        Camera3dBundle {
            projection: Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::WindowSize(40.0),
                ..Default::default()
            }),
            camera_3d: Camera3d::default(),
            camera: Camera {
                order: 2,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                hdr: true, // Transparent stuff fails to render properly if this is off - this may be a bevy bug?
                ..Default::default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        RenderLayers::from_layers(&[INVENTORY_SLOT_LAYER]),
    ));

    commands.spawn((
        Name::new("UI Middle Camera"),
        UiMiddleRoot,
        Camera3dBundle {
            projection: Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::WindowSize(40.0),
                ..Default::default()
            }),
            camera_3d: Camera3d::default(),
            camera: Camera {
                order: 1,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                hdr: true, // Transparent stuff fails to render properly if this is off - this may be a bevy bug?
                ..Default::default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        RenderLayers::from_layers(&[MIDDLE_INVENTORY_SLOT_LAYER]),
    ));
}

#[derive(Debug, Component, Reflect)]
/// Put this onto a UI element to render a 3D item there
pub struct RenderItem {
    /// The item's id
    pub item_id: u16,
}

#[derive(Eq, PartialEq, Default, Component, Debug, Clone, Copy, Reflect)]
/// For advanced item rendering, it may be required to have some UI elements behind it, and others in front of it.
pub enum ItemRenderLayer {
    #[default]
    /// The item should be rendered on the top-most item layer.
    ///
    /// Only UI elements targetting the [`UiTopRoot`] entity will be rendered over this.
    Top,
    /// The item will be rendered in the middle layer.
    ///
    /// All UI elements placed in the [`UiMiddleRoot`] and higher and anything rendered in higher order cameras will render over this.
    Middle,
}

#[derive(Debug, Component, Reflect)]
/// Represents an item that is currently rendered to the screen (in a UI).
pub struct RenderedItem {
    /// Points to the UI entity that had the `RenderItem` that created this
    ui_element_entity: Entity,
    item_id: u16,
    based_off: Vec3,
}

fn change_item_visibility(
    q_rendered_items: Query<(Entity, &RenderedItem)>,
    mut q_visibility: Query<(&mut Visibility, &mut ViewVisibility), With<RenderedItem>>,
    q_changed_view_visibility: Query<
        (Entity, &ViewVisibility, &Visibility),
        (
            Or<(Changed<ViewVisibility>, Changed<Visibility>)>,
            Without<RenderedItem>,
            With<RenderItem>,
        ),
    >,
) {
    for (entity, view_visibility, actual_vis) in &q_changed_view_visibility {
        let Some(rendered_item) = q_rendered_items
            .iter()
            .find(|(_, rendered_item)| rendered_item.ui_element_entity == entity)
        else {
            continue;
        };

        let rendered_item = rendered_item.0;

        let (mut cur_vis, mut vv) = q_visibility.get_mut(rendered_item).expect("Rendered item has no visibility - BAD!");

        if view_visibility.get() && actual_vis != Visibility::Hidden {
            *cur_vis = Visibility::Inherited;
            vv.set(); // sets it to visible
        } else {
            *cur_vis = Visibility::Hidden;
            *vv = ViewVisibility::HIDDEN;
        }
    }
}

fn render_items(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,

    block_items: Res<BlockItems>,
    items: Res<Registry<Item>>,
    blocks: Res<Registry<Block>>,

    block_materials_registry: Res<ManyToOneRegistry<Block, BlockMaterialMapping>>,
    block_textures: Res<Registry<BlockTextureIndex>>,
    block_meshes: Res<BlockMeshRegistry>,
    images: Res<Assets<Image>>,

    (
        mut q_transform,
        mut removed_render_items,
        changed_render_items,
        rendered_items,
        material_definitions_registry,
        mut event_writer,
        mut evw_remove_materials,
    ): (
        Query<&mut Transform>,
        RemovedComponents<RenderItem>,
        Query<
            (Entity, &RenderItem, &GlobalTransform, Option<&ItemRenderLayer>),
            Or<(Changed<RenderItem>, Changed<ItemRenderLayer>, Changed<GlobalTransform>)>,
        >,
        Query<(Entity, &RenderedItem)>,
        Res<Registry<MaterialDefinition>>,
        EventWriter<AddMaterialEvent>,
        EventWriter<RemoveAllMaterialsEvent>,
    ),

    item_materials_registry: Res<ManyToOneRegistry<Item, ItemMaterialMapping>>,
    atlas: Res<Registry<CosmosTextureAtlas>>,
    item_textures: Res<Registry<ItemTextureIndex>>,
) {
    for entity in removed_render_items.read() {
        if let Some((rendered_item_entity, _)) = rendered_items
            .iter()
            .find(|(_, rendered_item)| rendered_item.ui_element_entity == entity)
        {
            if let Some(mut ecmds) = commands.get_entity(rendered_item_entity) {
                ecmds.insert(NeedsDespawned);
            }
        }
    }

    for (entity, changed_render_item, transform, render_layer) in changed_render_items.iter() {
        let translation = transform.translation();

        let item = items.from_numeric_id(changed_render_item.item_id);

        let to_create = if let Some((rendered_item_entity, rendered_item)) = rendered_items
            .iter()
            .find(|(_, rendered_item)| rendered_item.ui_element_entity == entity)
        {
            if rendered_item.item_id == changed_render_item.item_id {
                // We're already displaying that item, no need to recalculate everything

                continue;
            }

            let mut transform = q_transform.get_mut(rendered_item_entity).expect("This must have a transform");

            transform.rotation = if block_items.block_from_item(item).is_some() {
                // This makes blocks look cool
                Quat::from_xyzw(0.07383737, 0.9098635, 0.18443844, 0.3642514)
            } else {
                Quat::from_axis_angle(Vec3::X, PI / 2.0)
            };

            rendered_item_entity
        } else {
            let mut transform = if block_items.block_from_item(item).is_some() {
                // This makes blocks look cool
                Transform::from_rotation(Quat::from_xyzw(0.07383737, 0.9098635, 0.18443844, 0.3642514))
            } else {
                Transform::from_rotation(Quat::from_axis_angle(Vec3::X, PI / 2.0))
            };

            // hide it till we position it properly
            transform.translation.x = -1000000.0;

            commands
                .spawn((TransformBundle::from_transform(transform), VisibilityBundle::default()))
                .id()
        };

        let render_layer = match render_layer {
            Some(ItemRenderLayer::Top) => INVENTORY_SLOT_LAYER,
            Some(ItemRenderLayer::Middle) | None => MIDDLE_INVENTORY_SLOT_LAYER,
        };

        // Clear out any materials that were previously on this entity from previous renders
        evw_remove_materials.send(RemoveAllMaterialsEvent { entity: to_create });

        if !generate_block_item_model(
            item,
            to_create,
            translation,
            entity,
            changed_render_item,
            &mut commands,
            &mut meshes,
            &block_items,
            &blocks,
            &block_materials_registry,
            &block_textures,
            &block_meshes,
            &material_definitions_registry,
            &mut event_writer,
            render_layer,
        ) {
            generate_item_model(
                item,
                to_create,
                translation,
                entity,
                changed_render_item,
                &mut commands,
                &mut meshes,
                &images,
                &item_materials_registry,
                &atlas,
                &item_textures,
                &material_definitions_registry,
                &mut event_writer,
                render_layer,
            );
        }
    }
}

fn generate_item_model(
    item: &Item,
    to_create: Entity,
    translation: Vec3,
    entity: Entity,
    changed_render_item: &RenderItem,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    images: &Assets<Image>,
    item_materials_registry: &ManyToOneRegistry<Item, ItemMaterialMapping>,
    atlas: &Registry<CosmosTextureAtlas>,
    item_textures: &Registry<ItemTextureIndex>,
    material_definitions_registry: &Registry<MaterialDefinition>,
    event_writer: &mut EventWriter<AddMaterialEvent>,
    render_layer: usize,
) {
    let size = 0.8;

    let index = item_textures
        .from_id(item.unlocalized_name())
        .unwrap_or_else(|| item_textures.from_id("missing").expect("Missing texture should exist."));

    let atlas = atlas.from_id("cosmos:main").unwrap();

    let image_index = index.atlas_index();

    let texture_data = SquareTextureAtlas::get_sub_image_data(
        images
            .get(
                atlas
                    .get_atlas_for_dimension_index(image_index.dimension_index)
                    .expect("Invalid dimension index passed!")
                    .get_atlas_handle(),
            )
            .expect("Missing atlas image"),
        image_index.texture_index,
    );

    let Some(item_material_mapping) = item_materials_registry.get_value(item) else {
        warn!("Missing material for block {}", item.unlocalized_name());
        return;
    };
    let mat_id = item_material_mapping.material_id();
    let material = material_definitions_registry.from_numeric_id(mat_id);

    let mesh = create_item_mesh(texture_data, item.id(), image_index.texture_index, material, size);
    let mesh_handle = meshes.add(mesh);

    commands.entity(to_create).insert((
        RenderedItem {
            based_off: translation,
            ui_element_entity: entity,
            item_id: changed_render_item.item_id,
        },
        mesh_handle,
        RenderLayers::from_layers(&[render_layer]),
        Name::new(format!("Rendered Inventory Item ({})", changed_render_item.item_id)),
    ));

    event_writer.send(AddMaterialEvent {
        entity: to_create,
        add_material_id: mat_id,
        texture_dimensions_index: image_index.dimension_index,
        material_type: MaterialType::Illuminated,
    });
}

fn generate_block_item_model(
    item: &Item,
    to_create: Entity,
    translation: Vec3,
    entity: Entity,
    changed_render_item: &RenderItem,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    block_items: &BlockItems,
    blocks: &Registry<Block>,
    block_materials_registry: &ManyToOneRegistry<Block, BlockMaterialMapping>,
    block_textures: &Registry<BlockTextureIndex>,
    block_meshes: &BlockMeshRegistry,
    material_definitions_registry: &Registry<MaterialDefinition>,
    event_writer: &mut EventWriter<AddMaterialEvent>,
    render_layer: usize,
) -> bool {
    let size = 0.8;

    let Some(block_id) = block_items.block_from_item(item) else {
        return false;
    };

    let block = blocks.from_numeric_id(block_id);

    let index = block_textures
        .from_id(block.unlocalized_name())
        .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

    let Some(block_mesh_info) = block_meshes.get_value(block) else {
        return false;
    };

    let mut mesh_builder = CosmosMeshBuilder::default();

    let Some(block_material_mapping) = block_materials_registry.get_value(block) else {
        warn!("Missing material for block {}", block.unlocalized_name());
        return false;
    };

    let mat_id = block_material_mapping.material_id();

    let material = material_definitions_registry.from_numeric_id(mat_id);

    let mut texture_dimensions_index = None;

    if block_mesh_info.has_multiple_face_meshes() {
        for face in [BlockFace::Top, BlockFace::Left, BlockFace::Front] {
            let Some(mut mesh_info) = block_mesh_info.info_for_face(face, false).cloned() else {
                break;
            };

            mesh_info.scale(Vec3::new(size, size, size));

            let Some(image_index) = index.atlas_index_from_face(face, BlockNeighbors::empty()) else {
                continue;
            };

            texture_dimensions_index = Some(image_index.dimension_index);

            mesh_builder.add_mesh_information(
                &mesh_info,
                Vec3::ZERO,
                Rect::new(0.0, 0.0, 1.0, 1.0),
                image_index.texture_index,
                material.add_material_data(block_id, &mesh_info),
            );
        }
    } else {
        let Some(mut mesh_info) = block_mesh_info.info_for_whole_block().cloned() else {
            return false;
        };

        mesh_info.scale(Vec3::new(size, size, size));

        let Some(image_index) = index.atlas_index_from_face(BlockFace::Front, BlockNeighbors::empty()) else {
            return false;
        };

        texture_dimensions_index = Some(image_index.dimension_index);

        mesh_builder.add_mesh_information(
            &mesh_info,
            Vec3::ZERO,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            image_index.texture_index,
            material.add_material_data(block_id, &mesh_info),
        );
    }

    let Some(texture_dimensions_index) = texture_dimensions_index else {
        return false;
    };

    commands.entity(to_create).insert((
        RenderedItem {
            based_off: translation,
            ui_element_entity: entity,
            item_id: changed_render_item.item_id,
        },
        meshes.add(mesh_builder.build_mesh()),
        RenderLayers::from_layers(&[render_layer]),
        Name::new(format!("Rendered Inventory Item ({})", changed_render_item.item_id)),
    ));

    event_writer.send(AddMaterialEvent {
        entity: to_create,
        add_material_id: mat_id,
        texture_dimensions_index,
        material_type: MaterialType::Illuminated,
    });

    true
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

        // if to avoid excess change detection
        if transform.translation.x != x {
            transform.translation.x = x;
        }
        if transform.translation.y != y {
            transform.translation.y = y;
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Add systems prior to this if you are having 3d items rendered to the screen and you don't want a 1-frame delay
///
/// Use the `RenderItem` component to render an item in a ui component.
pub enum RenderItemSystemSet {
    /// Turn the `RenderItem` component into an actual UI component on your screen
    RenderItems,
}

// fn print_quat(query: Query<&Transform, (Changed<Transform>, With<RenderedItem>)>) {
//     for trans in query.iter() {
//         println!(
//             "{}, {}, {}, {}",
//             trans.rotation.x, trans.rotation.y, trans.rotation.z, trans.rotation.w
//         );
//     }
// }

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (RenderItemSystemSet::RenderItems.in_set(MaterialsSystemSet::RequestMaterialChanges))
            .chain()
            .in_set(UiSystemSet::DoUi),
    )
    .add_systems(
        Update,
        ((
            update_rendered_items_transforms,
            reposition_ui_items,
            render_items,
            change_item_visibility,
        )
            .chain()
            .in_set(RenderItemSystemSet::RenderItems),),
    );

    app.add_systems(OnEnter(GameState::Playing), create_ui_cameras)
        .register_type::<RenderItem>()
        .register_type::<RenderedItem>()
        .register_type::<ItemRenderLayer>();
}
