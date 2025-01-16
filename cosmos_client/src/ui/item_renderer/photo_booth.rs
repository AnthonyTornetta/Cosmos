use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        camera::ScalingMode,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers,
    },
};
use cosmos_core::{
    blockitems::BlockItems,
    item::Item,
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    utils::array_utils::expand_2d,
};

use crate::{
    asset::materials::{AddMaterialEvent, MaterialType},
    item::item_mesh::ItemMeshMaterial,
};

const PHOTO_BOOTH_RENDER_LAYER: usize = 0b100000;

const PX_PER_ITEM: usize = 100;

#[derive(Resource, Debug)]
/// Contains a rendered view of every item (and block in item-form) in the game. This is used for
/// GUI rendering.
///
/// You probably don't need to use this directly, and should instead use [`super::RenderItem`] if
/// possible.
pub struct RenderedItemAtlas {
    width: usize,
    handle: Handle<Image>,
}

impl RenderedItemAtlas {
    /// Returns the rect that contains this specific item's rendered model within the
    /// [`Self::get_atlas_handle`].
    ///
    /// You probably don't need to call this directly, and should instead use [`super::RenderItem`]
    pub fn get_item_rect(&self, item: &Item) -> Rect {
        let (x, y) = expand_2d(item.id() as usize, self.width);
        let (x, y) = (x as f32, y as f32);
        const PPI: f32 = PX_PER_ITEM as f32;

        Rect {
            min: Vec2::new(PPI * x, PPI * y),
            max: Vec2::new(PPI * (x + 1.0), PPI * (y + 1.0)),
        }
    }

    /// Returns the image handle for the entire item atlas.
    ///
    /// You probably don't need to call this directly, and should instead use [`super::RenderItem`]
    pub fn get_atlas_handle(&self) -> &Handle<Image> {
        &self.handle
    }
}

fn setup_rendered_item_atlas(mut images: ResMut<Assets<Image>>, w: usize, h: usize) -> Handle<Image> {
    let size = Extent3d {
        width: w as u32,
        height: h as u32,
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    // You need to set these texture usage flags in order to use the image as a render target
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    images.add(image)
}

fn create_booth(
    mut commands: Commands,
    mut event_writer: EventWriter<AddMaterialEvent>,
    items: Res<Registry<Item>>,
    item_meshes: Res<Registry<ItemMeshMaterial>>,
    block_items: Res<BlockItems>,
    images: ResMut<Assets<Image>>,
) {
    let n_items = items.iter().len();

    let w = (n_items as f32).sqrt().ceil();
    let h = (n_items as f32 / w).ceil();

    let width = w as usize;
    let height = h as usize;

    let image_handle = setup_rendered_item_atlas(images, PX_PER_ITEM * width, PX_PER_ITEM * height);

    const GAP: f32 = 2.0;
    let cam_w = w * GAP;
    let cam_h = h * GAP;

    commands.spawn((
        Name::new("Photo Booth Camera"),
        Camera3d { ..Default::default() },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: cam_w,
                height: cam_h,
            },
            ..OrthographicProjection::default_3d()
        }),
        Camera {
            order: 10,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            target: image_handle.clone().into(),
            hdr: true, // Transparent stuff fails to render properly if this is off - this may be a bevy bug?
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0)
            .looking_at(Vec3::ZERO, Vec3::Y)
            .with_translation(Vec3::new(cam_w / 2.0 - 1.0, cam_h / 2.0 - 1.0, 1.0)),
        RenderLayers::from_layers(&[PHOTO_BOOTH_RENDER_LAYER]),
    ));

    // Uncomment to debug the rendered items + blocks:
    // commands
    //     .spawn((
    //         Name::new("rendered image"),
    //         Node {
    //             width: Val::Percent(100.0),
    //             height: Val::Percent(100.0),
    //             ..Default::default()
    //         },
    //     ))
    //     .with_children(|p| {
    //         p.spawn(ImageNode {
    //             image: image_handle.clone(),
    //             ..Default::default()
    //         });
    //     });

    commands.insert_resource(RenderedItemAtlas {
        width,
        handle: image_handle,
    });

    for (i, item) in items.iter().enumerate() {
        let Some(item_mat_material) = item_meshes.from_id(item.unlocalized_name()) else {
            info!("{item_meshes:?}");
            warn!("Missing rendering material for item {}", item.unlocalized_name());
            return;
        };

        let (x, y) = expand_2d(i, width);

        let rot = if block_items.block_from_item(item).is_some() {
            // This makes blocks look cool
            Quat::from_xyzw(0.07383737, 0.9098635, 0.18443844, 0.3642514)
        } else {
            Quat::from_axis_angle(Vec3::X, PI / 2.0)
        };

        let entity = commands
            .spawn((
                RenderLayers::from_layers(&[PHOTO_BOOTH_RENDER_LAYER]),
                Mesh3d(item_mat_material.mesh_handle().clone_weak()),
                // h - y - 1 because we want low IDs at the top, and big IDs at the bottom (and +y
                // is up in this context)
                Transform::from_xyz(x as f32 * GAP, (h - y as f32 - 1.0) * GAP, 0.0).with_rotation(rot),
            ))
            .id();

        event_writer.send(AddMaterialEvent {
            entity,
            add_material_id: item_mat_material.material_id(),
            texture_dimensions_index: item_mat_material.texture_dimension_index(),
            material_type: MaterialType::Illuminated,
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::LoadingWorld), create_booth);
}
