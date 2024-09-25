//! Mesh-creation logic for items

use bevy::{
    asset::{Assets, Handle},
    color::Srgba,
    log::warn,
    math::{Rect, Vec2, Vec3},
    prelude::{App, Image, IntoSystemConfigs, OnExit, Res, ResMut},
    render::mesh::Mesh,
};
use cosmos_core::{
    block::{
        block_face::{BlockFace, ALL_BLOCK_FACES},
        Block,
    },
    blockitems::BlockItems,
    item::Item,
    registry::{create_registry, identifiable::Identifiable, many_to_one::ManyToOneRegistry, Registry},
    state::GameState,
    utils::array_utils::{expand_2d, flatten_2d},
};

use crate::asset::asset_loading::BlockNeighbors;
use crate::{
    asset::{
        asset_loading::{BlockTextureIndex, ItemMeshingLoadingSet},
        materials::BlockMaterialMapping,
    },
    rendering::BlockMeshRegistry,
};
use crate::{
    asset::{
        asset_loading::{CosmosTextureAtlas, ItemTextureIndex},
        materials::{ItemMaterialMapping, MaterialDefinition},
        texture_atlas::SquareTextureAtlas,
    },
    rendering::{CosmosMeshBuilder, MeshBuilder, MeshInformation},
};

#[derive(Clone, Debug)]
/// Contains the information required to render an item.
///
/// This also includes the information on how to render items that are blocks.
pub struct ItemMeshMaterial {
    id: u16,
    unlocalized_name: String,
    handle: Handle<Mesh>,
    material_id: u16,
    dimension_index: u32,
}

impl ItemMeshMaterial {
    /// Returns the handle to this item's mesh
    pub fn mesh_handle(&self) -> &Handle<Mesh> {
        &self.handle
    }

    /// Returns the id of the material this item uses.
    ///
    /// Used in the [`Registry<MaterialDefinition>`].
    pub fn material_id(&self) -> u16 {
        self.material_id
    }

    /// Returns the dimension_index (from [`crate::asset::asset_loading::TextureIndex`]) this item uses.
    pub fn texture_dimension_index(&self) -> u32 {
        self.dimension_index
    }
}

impl Identifiable for ItemMeshMaterial {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        self.unlocalized_name.as_str()
    }
}

fn generate_item_model(
    item: &Item,
    images: &Assets<Image>,
    item_materials_registry: &ManyToOneRegistry<Item, ItemMaterialMapping>,
    atlas: &Registry<CosmosTextureAtlas>,
    item_textures: &Registry<ItemTextureIndex>,
    material_definitions_registry: &Registry<MaterialDefinition>,
) -> Option<(Mesh, u16, u32)> {
    println!("{item_textures:?}");
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

    let item_material_mapping = item_materials_registry.get_value(item)?;
    let mat_id = item_material_mapping.material_id();
    let material = material_definitions_registry.from_numeric_id(mat_id);

    let mesh = create_item_mesh(texture_data, item.id(), image_index.texture_index, material, 1.0);

    Some((mesh, mat_id, image_index.dimension_index))
}

fn create_item_meshes(
    block_items: Res<BlockItems>,
    items: Res<Registry<Item>>,
    blocks: Res<Registry<Block>>,
    mut registry: ResMut<Registry<ItemMeshMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    item_materials_registry: Res<ManyToOneRegistry<Item, ItemMaterialMapping>>,
    atlas: Res<Registry<CosmosTextureAtlas>>,
    item_textures: Res<Registry<ItemTextureIndex>>,
    material_definitions_registry: Res<Registry<MaterialDefinition>>,
    block_materials_registry: Res<ManyToOneRegistry<Block, BlockMaterialMapping>>,
    block_textures: Res<Registry<BlockTextureIndex>>,
    block_meshes: Res<BlockMeshRegistry>,
) {
    for item in items.iter() {
        // Don't override existing models
        if registry.contains(item.unlocalized_name()) {
            continue;
        }

        let (mesh, material_id, dimension_index) = if let Some(block_id) = block_items.block_from_item(item) {
            let block = blocks.from_numeric_id(block_id);

            let Some(x) = generate_block_item_model(
                block,
                &block_materials_registry,
                &block_textures,
                &block_meshes,
                &material_definitions_registry,
            ) else {
                continue;
            };

            x
        } else {
            let Some(x) = generate_item_model(
                item,
                &images,
                &item_materials_registry,
                &atlas,
                &item_textures,
                &material_definitions_registry,
            ) else {
                warn!("Got no item model for {item:?}");
                continue;
            };

            x
        };

        let mesh_handle = meshes.add(mesh);

        registry.register(ItemMeshMaterial {
            id: 0,
            unlocalized_name: item.unlocalized_name().to_owned(),
            handle: mesh_handle,
            material_id,
            dimension_index,
        });
    }
}

/// Creates a mesh for an item based on its image data.
fn create_item_mesh(square_image_data: &[u8], item_id: u16, image_index: u32, mat: &MaterialDefinition, scale: f32) -> Mesh {
    // Data is assumed to be a square image
    let w = ((square_image_data.len() / 4) as f32).sqrt() as usize;
    let h = w;

    let mut pixels = vec![None; w * h];

    // let pixel_size = 1.0 / w as f32;
    let pixel_height = 1.0 / 16.0;

    for y in 0..h {
        for x in 0..w {
            let data_idx = flatten_2d(x * 4, y, w * 4);
            let rgba = &square_image_data[data_idx..(data_idx + 4)];
            let r = rgba[0];
            let g = rgba[1];
            let b = rgba[2];
            let a = rgba[3];

            if a == 0 {
                continue;
            }

            pixels[flatten_2d(x, y, w)] = Some(Srgba {
                red: r as f32 / u8::MAX as f32,
                green: g as f32 / u8::MAX as f32,
                blue: b as f32 / u8::MAX as f32,
                alpha: a as f32 / u8::MAX as f32,
            });
        }
    }

    let mut cmbuilder = CosmosMeshBuilder::default();

    let ph = pixel_height / 2.0;

    for (idx, _) in pixels.iter().enumerate().filter(|(_, x)| x.is_some()) {
        let (x, y) = expand_2d(idx, w);

        let min = Vec2::new(x as f32 / w as f32, y as f32 / h as f32);
        let max = Vec2::new((x + 1) as f32 / w as f32, (y + 1) as f32 / h as f32);

        let pmin = min - Vec2::new(0.5, 0.5);
        let pmax = max - Vec2::new(0.5, 0.5);

        let mut info = MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
            positions: vec![
                [pmax.x, ph, pmin.y],
                [pmin.x, ph, pmin.y],
                [pmin.x, ph, pmax.y],
                [pmax.x, ph, pmax.y],
            ],
            normals: [[0.0, 1.0, 0.0]; 4].to_vec(),
        };

        info.scale(Vec3::splat(scale));

        cmbuilder.add_mesh_information(
            &info,
            Vec3::ZERO,
            Rect::from_corners(min, max),
            image_index,
            mat.add_item_material_data(item_id, &info),
        );

        let mut info = MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            positions: vec![
                [pmax.x, -ph, pmax.y],
                [pmin.x, -ph, pmax.y],
                [pmin.x, -ph, pmin.y],
                [pmax.x, -ph, pmin.y],
            ],
            normals: [[0.0, -1.0, 0.0]; 4].to_vec(),
        };

        info.scale(Vec3::splat(scale));

        cmbuilder.add_mesh_information(
            &info,
            Vec3::ZERO,
            Rect::from_corners(min, max),
            image_index,
            mat.add_item_material_data(item_id, &info),
        );

        if x == 0 || pixels[flatten_2d(x - 1, y, w)].is_none() {
            // draw left square
            let mut info = MeshInformation {
                indices: vec![0, 1, 2, 2, 3, 0],
                uvs: vec![[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
                positions: vec![
                    [pmin.x, -ph, pmax.y],
                    [pmin.x, ph, pmax.y],
                    [pmin.x, ph, pmin.y],
                    [pmin.x, -ph, pmin.y],
                ],
                normals: [[-1.0, 0.0, 0.0]; 4].to_vec(),
            };
            info.scale(Vec3::splat(scale));

            cmbuilder.add_mesh_information(
                &info,
                Vec3::ZERO,
                Rect::from_corners(min, max),
                image_index,
                mat.add_item_material_data(item_id, &info),
            );
        }
        if x + 1 == w || pixels[flatten_2d(x + 1, y, w)].is_none() {
            // draw right square
            let mut info = MeshInformation {
                indices: vec![0, 1, 2, 2, 3, 0],
                uvs: vec![[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
                positions: vec![
                    [pmax.x, -ph, pmin.y],
                    [pmax.x, ph, pmin.y],
                    [pmax.x, ph, pmax.y],
                    [pmax.x, -ph, pmax.y],
                ],
                normals: [[1.0, 0.0, 0.0]; 4].to_vec(),
            };

            info.scale(Vec3::splat(scale));

            cmbuilder.add_mesh_information(
                &info,
                Vec3::ZERO,
                Rect::from_corners(min, max),
                image_index,
                mat.add_item_material_data(item_id, &info),
            );
        }
        if y == 0 || pixels[flatten_2d(x, y - 1, w)].is_none() {
            // draw top square
            let mut info = MeshInformation {
                indices: vec![0, 1, 2, 2, 3, 0],
                uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                positions: vec![
                    [pmin.x, ph, pmin.y],
                    [pmax.x, ph, pmin.y],
                    [pmax.x, -ph, pmin.y],
                    [pmin.x, -ph, pmin.y],
                ],
                normals: [[0.0, 0.0, -1.0]; 4].to_vec(),
            };

            info.scale(Vec3::splat(scale));

            cmbuilder.add_mesh_information(
                &info,
                Vec3::ZERO,
                Rect::from_corners(min, max),
                image_index,
                mat.add_item_material_data(item_id, &info),
            );
        }
        if y + 1 == h || pixels[flatten_2d(x, y + 1, w)].is_none() {
            // draw bottom square
            let mut info = MeshInformation {
                indices: vec![0, 1, 2, 2, 3, 0],
                uvs: vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
                positions: vec![
                    [pmin.x, -ph, pmax.y],
                    [pmax.x, -ph, pmax.y],
                    [pmax.x, ph, pmax.y],
                    [pmin.x, ph, pmax.y],
                ],
                normals: [[0.0, 0.0, 1.0]; 4].to_vec(),
            };

            info.scale(Vec3::splat(scale));

            cmbuilder.add_mesh_information(
                &info,
                Vec3::ZERO,
                Rect::from_corners(min, max),
                image_index,
                mat.add_item_material_data(item_id, &info),
            );
        }
    }

    cmbuilder.build_mesh()
}

fn generate_block_item_model(
    block: &Block,
    block_materials_registry: &ManyToOneRegistry<Block, BlockMaterialMapping>,
    block_textures: &Registry<BlockTextureIndex>,
    block_meshes: &BlockMeshRegistry,
    material_definitions_registry: &Registry<MaterialDefinition>,
) -> Option<(Mesh, u16, u32)> {
    let index = block_textures
        .from_id(block.unlocalized_name())
        .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

    let block_mesh_info = block_meshes.get_value(block)?;

    let mut mesh_builder = CosmosMeshBuilder::default();

    let Some(block_material_mapping) = block_materials_registry.get_value(block) else {
        warn!("Missing material for block {}", block.unlocalized_name());
        return None;
    };

    let mat_id = block_material_mapping.material_id();

    let material = material_definitions_registry.from_numeric_id(mat_id);

    let dimension_index = if block_mesh_info.has_multiple_face_meshes() {
        let mut texture_dims = None;
        for face in ALL_BLOCK_FACES {
            let Some(mesh_info) = block_mesh_info.info_for_face(face, false) else {
                break;
            };

            let Some(image_index) = index.atlas_index_from_face(face, BlockNeighbors::empty()) else {
                continue;
            };

            if let Some(td) = texture_dims {
                if td != image_index.dimension_index {
                    panic!("Block contains textures with different dimensions on different faces!");
                }
            } else {
                texture_dims = Some(image_index.dimension_index)
            }

            mesh_builder.add_mesh_information(
                mesh_info,
                Vec3::ZERO,
                Rect::new(0.0, 0.0, 1.0, 1.0),
                image_index.texture_index,
                material.add_material_data(block.id(), mesh_info),
            );
        }

        texture_dims.expect("Set above")
    } else {
        let mesh_info = block_mesh_info.info_for_whole_block()?;
        let image_index = index.atlas_index_from_face(BlockFace::Front, BlockNeighbors::empty())?;

        mesh_builder.add_mesh_information(
            mesh_info,
            Vec3::ZERO,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            image_index.texture_index,
            material.add_material_data(block.id(), mesh_info),
        );

        image_index.dimension_index
    };

    let mesh = mesh_builder.build_mesh();

    // To think: do I want to support the same block having multiple faces w/ different texture
    // dimensions? Seems kinda pointless.

    Some((mesh, mat_id, dimension_index))
}

pub(super) fn register(app: &mut App) {
    create_registry::<ItemMeshMaterial>(app, "cosmos:item_mesh_material");

    app.add_systems(
        OnExit(GameState::PostLoading),
        create_item_meshes.in_set(ItemMeshingLoadingSet::GenerateMeshes),
    );
}
