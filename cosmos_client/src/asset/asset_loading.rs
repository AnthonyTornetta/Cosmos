use std::fs;

use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    utils::HashMap,
};
use cosmos_core::{
    block::{Block, BlockFace},
    loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager},
    registry::{self, identifiable::Identifiable, Registry},
};
use serde::{Deserialize, Serialize};

use crate::state::game_state::GameState;

enum AtlasName {
    Main,
}

#[derive(Resource)]
struct LoadingAsset {
    atlas_name: AtlasName,
    handles: Vec<Handle<Image>>,
}

struct AssetsDoneLoadingEvent;

#[derive(Resource)]
struct AssetsLoadingID(usize);

#[derive(Resource)]
struct AssetsLoading(Vec<LoadingAsset>);

#[derive(Resource, Reflect, FromReflect)]
pub struct MainAtlas {
    pub material: Handle<StandardMaterial>,
    pub unlit_material: Handle<StandardMaterial>,
    pub atlas: TextureAtlas,

    padding: u32,
}

impl MainAtlas {
    #[inline]
    pub fn uvs_for_index(&self, index: usize) -> Rect {
        let rect = self.atlas.textures[index];

        let padding_x = self.padding as f32 / self.atlas.size.x;
        let padding_y = self.padding as f32 / self.atlas.size.y;

        Rect::new(
            rect.min.x / self.atlas.size.x + padding_x,
            rect.min.y / self.atlas.size.y + padding_y,
            rect.max.x / self.atlas.size.x - padding_x,
            rect.max.y / self.atlas.size.y - padding_y,
        )
    }
}

#[derive(Resource, Reflect, FromReflect)]
pub struct IlluminatedAtlas {
    pub material: Handle<StandardMaterial>,
}

fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut loading: ResMut<AssetsLoading>,
    mut loader: ResMut<LoadingManager>,
    mut start_writer: EventWriter<AddLoadingEvent>,
) {
    loading.0.push(LoadingAsset {
        handles: server
            .load_folder("images/blocks/")
            .expect("error loading blocks textures")
            .into_iter()
            .map(|x| x.typed::<Image>())
            .collect(),
        atlas_name: AtlasName::Main,
    });

    commands.insert_resource(AssetsLoadingID(loader.register_loader(&mut start_writer)));
}

fn assets_done_loading(
    mut commands: Commands,
    event_listener: EventReader<AssetsDoneLoadingEvent>,
    loading_id: Option<Res<AssetsLoadingID>>,
    mut loader: ResMut<LoadingManager>,
    mut end_writer: EventWriter<DoneLoadingEvent>,
) {
    if loading_id.is_some() && !event_listener.is_empty() {
        loader.finish_loading(loading_id.as_ref().unwrap().0, &mut end_writer);

        commands.remove_resource::<AssetsLoadingID>();
    }
}

fn expand_image(image: &Image, padding: u32) -> Image {
    let mut data: Vec<u8> = Vec::new();

    let mut i = 0;

    let image_size_x = image.size().x as u32;
    let image_size_y = image.size().y as u32;

    for y in 0..image_size_y as usize {
        let mut n = match y % image_size_y as usize == 0 || (y + 1) % image_size_y as usize == 0 {
            true => 1 + padding,
            false => 1,
        };

        while n > 0 {
            let og_i = i;

            for x in 0..image_size_x as usize {
                if x % image_size_x as usize == 0 || (x + 1) % image_size_x as usize == 0 {
                    for _ in 0..(padding + 1) {
                        data.push(image.data[i]);
                        data.push(image.data[i + 1]);
                        data.push(image.data[i + 2]);
                        data.push(image.data[i + 3]);
                    }
                } else {
                    data.push(image.data[i]);
                    data.push(image.data[i + 1]);
                    data.push(image.data[i + 2]);
                    data.push(image.data[i + 3]);
                }

                i += 4;
            }

            n -= 1;

            if n != 0 {
                i = og_i;
            }
        }
    }

    let height = image_size_y + padding * 2;
    let width = image_size_y + padding * 2;

    // debug save
    // image::save_buffer(&Path::new("image.png"), data.as_slice(), width, height, image::ColorType::Rgba8);

    Image::new(
        Extent3d {
            height,
            width,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
    )
}

fn check_assets_ready(
    mut commands: Commands,
    server: Res<AssetServer>,
    loading: Option<ResMut<AssetsLoading>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut event_writer: EventWriter<AssetsDoneLoadingEvent>,
) {
    let Some(loading) = loading else {
        return;
    };

    use bevy::asset::LoadState;

    let mut handles = Vec::new();
    for la in loading.0.iter().map(|h| &h.handles) {
        for handle in la.iter() {
            handles.push(handle.id());
        }
    }

    match server.get_group_load_state(handles) {
        LoadState::Failed => {
            panic!("Failed to load asset!!");
        }
        LoadState::Loaded => {
            // all assets are now ready, construct texture atlas
            // for better performance

            for asset in loading.0.iter() {
                match asset.atlas_name {
                    AtlasName::Main => {
                        const PADDING: u32 = 2;

                        let mut texture_atlas_builder = TextureAtlasBuilder::default();

                        for handle in &asset.handles {
                            let Some(image) = images.get(handle) else {
                                warn!("{:?} did not resolve to an `Image` asset.", server.get_handle_path(handle));
                                continue;
                            };

                            let img = expand_image(image, PADDING);

                            let handle = images.set(handle.clone(), img);

                            texture_atlas_builder.add_texture(
                                handle.clone_weak(),
                                images
                                    .get(&handle)
                                    .expect("This image was just added, but doesn't exist."),
                            );
                        }

                        let atlas = texture_atlas_builder
                            .finish(&mut images)
                            .expect("Failed to build atlas");

                        let material_handle = materials.add(StandardMaterial {
                            base_color_texture: Some(atlas.texture.clone()),
                            alpha_mode: AlphaMode::Mask(0.5),
                            unlit: false,
                            metallic: 0.0,
                            reflectance: 0.0,

                            ..default()
                        });

                        let unlit_material_handle = materials.add(StandardMaterial {
                            base_color_texture: Some(atlas.texture.clone()),
                            alpha_mode: AlphaMode::Mask(0.5),
                            unlit: true,
                            metallic: 0.0,
                            reflectance: 0.0,

                            ..default()
                        });

                        let texture = atlas.texture.clone();

                        commands.insert_resource(MainAtlas {
                            material: material_handle,
                            unlit_material: unlit_material_handle,
                            atlas,
                            padding: PADDING,
                        });

                        let illuminated_material_handle = materials.add(StandardMaterial {
                            base_color_texture: Some(texture),
                            alpha_mode: AlphaMode::Mask(0.5),
                            unlit: true,
                            double_sided: true,
                            ..default()
                        });

                        commands.insert_resource(IlluminatedAtlas {
                            material: illuminated_material_handle,
                        });
                    }
                }
            }

            // Clear out handles to avoid continually checking
            commands.remove_resource::<AssetsLoading>();

            // (note: if you don't have any other handles to the assets
            // elsewhere, they will get unloaded after this)

            event_writer.send(AssetsDoneLoadingEvent);
        }
        _ => {
            // NotLoaded/Loading: not fully ready yet
        }
    }
}

#[derive(Debug)]
pub struct BlockTextureIndicies(HashMap<String, usize>);

impl BlockTextureIndicies {
    pub fn all(index: usize) -> Self {
        let mut map = HashMap::new();
        map.insert("all".into(), index);
        Self(map)
    }

    pub fn new(map: HashMap<String, usize>) -> Self {
        Self(map)
    }
}

#[derive(Debug)]
pub struct BlockTextureIndex {
    indices: BlockTextureIndicies,
    id: u16,
    unlocalized_name: String,
}

impl BlockTextureIndex {
    #[inline]
    pub fn atlas_index_from_face(&self, face: BlockFace) -> Option<usize> {
        self.atlas_index(face.as_str())
    }

    #[inline]
    pub fn atlas_index(&self, identifier: &str) -> Option<usize> {
        if let Some(index) = self.indices.0.get(identifier) {
            Some(*index)
        } else {
            self.indices.0.get("all").copied()
        }
    }
}

impl Identifiable for BlockTextureIndex {
    #[inline]
    fn id(&self) -> u16 {
        self.id
    }

    #[inline]
    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    #[inline]
    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct BlockInfo {
    texture: HashMap<String, String>,
}

fn load_block_textxures(
    blocks: Res<Registry<Block>>,
    atlas: Res<MainAtlas>,
    server: Res<AssetServer>,
    mut registry: ResMut<Registry<BlockTextureIndex>>,
) {
    if let Some(index) = atlas
        .atlas
        .get_texture_index(&server.get_handle("images/blocks/missing.png"))
    {
        registry.register(BlockTextureIndex {
            id: 0,
            unlocalized_name: "missing".to_owned(),
            indices: BlockTextureIndicies::all(index),
        });
    }

    for block in blocks.iter() {
        let unlocalized_name = block.unlocalized_name();
        let block_name = unlocalized_name
            .split(':')
            .nth(1)
            .unwrap_or(unlocalized_name);

        let json_path = format!("assets/blocks/{block_name}.json");

        let block_info = if let Ok(block_info) = fs::read(&json_path) {
            

            serde_json::from_slice::<BlockInfo>(&block_info)
                .unwrap_or_else(|_| panic!("Error reading json data in {json_path}"))
        } else {
            let mut hh = HashMap::new();
            hh.insert("all".into(), block_name.to_owned());
            BlockInfo { texture: hh }
        };

        let mut map = HashMap::new();
        for (entry, texture_name) in block_info.texture.iter() {
            if let Some(index) = atlas.atlas.get_texture_index(
                &server.get_handle(&format!("images/blocks/{texture_name}.png",)),
            ) {
                map.insert(entry.to_owned(), index);
            }
        }

        registry.register(BlockTextureIndex {
            id: 0,
            unlocalized_name: unlocalized_name.to_owned(),
            indices: BlockTextureIndicies::new(map),
        });
    }
}

pub fn register(app: &mut App) {
    registry::create_registry::<BlockTextureIndex>(app);

    app.insert_resource(AssetsLoading(Vec::new()))
        .add_event::<AssetsDoneLoadingEvent>()
        .add_systems(
            (check_assets_ready, assets_done_loading).in_set(OnUpdate(GameState::PostLoading)),
        )
        .add_system(setup.in_schedule(OnEnter(GameState::PostLoading)))
        .add_system(load_block_textxures.in_schedule(OnExit(GameState::PostLoading)));
}
