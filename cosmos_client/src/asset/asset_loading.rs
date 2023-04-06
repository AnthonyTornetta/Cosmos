use std::fs;

use bevy::{prelude::*, utils::HashMap};
use cosmos_core::{
    block::{Block, BlockFace},
    loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager},
    registry::{self, identifiable::Identifiable, Registry},
};
use serde::{Deserialize, Serialize};

use crate::{rendering::uv_mapper::UVMapper, state::game_state::GameState};

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
    pub atlas: TextureAtlas,
    pub uv_mapper: UVMapper,
}

impl MainAtlas {
    pub fn uvs_for_index(&self, index: usize) -> Rect {
        let rect = self.atlas.textures[index];

        Rect::new(
            rect.min.x / self.atlas.size.x,
            rect.min.y / self.atlas.size.y,
            rect.max.x / self.atlas.size.x,
            rect.max.y / self.atlas.size.y,
        )
    }
}

#[derive(Resource, Reflect, FromReflect)]
pub struct IlluminatedAtlas {
    pub material: Handle<StandardMaterial>,
    pub uv_mapper: UVMapper,
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
            // all assets are now ready

            for asset in loading.0.iter() {
                match asset.atlas_name {
                    AtlasName::Main => {
                        const PADDING: u32 = 0;
                        const IMAGE_DIMENSIONS: u32 = 16;

                        let mut texture_atlas_builder = TextureAtlasBuilder::default();

                        for handle in &asset.handles {
                            println!("doing {:?}", server.get_handle_path(handle));
                            let Some(texture) = images.get(&handle) else {
                                warn!("{:?} did not resolve to an `Image` asset.", server.get_handle_path(handle));
                                continue;
                            };

                            texture_atlas_builder.add_texture(handle.clone_weak(), texture);
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

                        let (width, height) = (atlas.size.x as usize, atlas.size.y as usize);

                        let texture = atlas.texture.clone();

                        commands.insert_resource(MainAtlas {
                            //handle,
                            material: material_handle,
                            atlas,
                            uv_mapper: UVMapper::new(
                                width,
                                height,
                                IMAGE_DIMENSIONS as usize,
                                IMAGE_DIMENSIONS as usize,
                                PADDING as usize,
                                PADDING as usize,
                            ),
                        });

                        let illuminated_material_handle = materials.add(StandardMaterial {
                            base_color_texture: Some(texture),
                            alpha_mode: AlphaMode::Mask(0.5),
                            unlit: true,
                            // emissive: Color::rgba_linear(1.0, 1.0, 1.0, 0.1),
                            double_sided: true,
                            ..default()
                        });

                        commands.insert_resource(IlluminatedAtlas {
                            material: illuminated_material_handle,
                            uv_mapper: UVMapper::new(
                                width,
                                height,
                                IMAGE_DIMENSIONS as usize,
                                IMAGE_DIMENSIONS as usize,
                                PADDING as usize,
                                PADDING as usize,
                            ),
                        })
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
            .split(":")
            .nth(1)
            .unwrap_or(unlocalized_name);

        let json_path = format!("assets/blocks/{block_name}.json");

        let block_info = if let Ok(block_info) = fs::read(&json_path) {
            let res = serde_json::from_slice::<BlockInfo>(&block_info)
                .expect(&format!("Error reading json data in {json_path}"));

            println!("{res:?}");

            res
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

        println!("{unlocalized_name}: {map:?}");

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
