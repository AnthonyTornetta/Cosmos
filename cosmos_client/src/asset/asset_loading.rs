//! Handles the loading of all texture assets.
//!
//! This also combines the textures into one big atlas.

use std::fs;

use bevy::{
    asset::{LoadState, LoadedFolder, RecursiveDependencyLoadState},
    prelude::*,
    utils::HashMap,
};
use bitflags::bitflags;
use cosmos_core::{
    block::{Block, BlockFace},
    loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager},
    registry::{self, identifiable::Identifiable, Registry},
};
use serde::{Deserialize, Serialize};

use crate::{asset::texture_atlas::SquareTextureAtlasBuilder, state::game_state::GameState};

use super::texture_atlas::SquareTextureAtlas;

#[derive(Resource, Debug, Clone)]
struct LoadingTextureAtlas {
    unlocalized_name: String,
    id: u16,
    handles: Handle<LoadedFolder>,
}

impl Identifiable for LoadingTextureAtlas {
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

impl LoadingTextureAtlas {
    pub fn new(unlocalized_name: impl Into<String>, handles: Handle<LoadedFolder>) -> Self {
        Self {
            handles,
            id: 0,
            unlocalized_name: unlocalized_name.into(),
        }
    }
}

#[derive(Debug, Event)]
/// Send this whenever you register a loader and want to signify that your assets are done loading
pub struct AssetsDoneLoadingEvent;

#[derive(Debug, Event)]
/// Sent whenever all the textures are done being loaded into the `CosmosTextureAtlas`
pub struct AllTexturesDoneLoadingEvent;

#[derive(Resource, Debug)]
struct AssetsLoadingID(usize);

fn setup_textures(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut loading: ResMut<Registry<LoadingTextureAtlas>>,
    mut loader: ResMut<LoadingManager>,
    mut start_writer: EventWriter<AddLoadingEvent>,
) {
    let image_handles = server.load_folder("cosmos/images/blocks/");
    // .expect("error loading blocks textures")
    // .into_iter()
    // .map(|x| x.typed::<Image>())
    // .collect();

    loading.register(LoadingTextureAtlas::new("cosmos:main", image_handles));

    commands.insert_resource(AssetsLoadingID(loader.register_loader(&mut start_writer)));
}

fn assets_done_loading(
    mut commands: Commands,
    event_listener: EventReader<AssetsDoneLoadingEvent>,
    loading_id: Option<Res<AssetsLoadingID>>,
    mut loader: ResMut<LoadingManager>,
    mut end_writer: EventWriter<DoneLoadingEvent>,
) {
    if !event_listener.is_empty() {
        if let Some(loading_id) = loading_id.as_ref() {
            loader.finish_loading(loading_id.0, &mut end_writer);

            commands.remove_resource::<AssetsLoadingID>();
        }
    }
}

#[derive(Clone, Debug, Reflect)]
/// A newtype wrapper around a bevy `TextureAtlas`
pub struct CosmosTextureAtlas {
    /// The texture atlas
    pub texture_atlas: SquareTextureAtlas,
    unlocalized_name: String,
    id: u16,
}

impl CosmosTextureAtlas {
    /// Creates a new Cosmos texture atlas - a newtype wrapper around a bevy `TextureAtlas`
    pub fn new(unlocalized_name: impl Into<String>, atlas: SquareTextureAtlas) -> Self {
        Self {
            unlocalized_name: unlocalized_name.into(),
            id: 0,
            texture_atlas: atlas,
        }
    }
}

impl Identifiable for CosmosTextureAtlas {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

fn check_assets_ready(
    mut commands: Commands,
    server: Res<AssetServer>,
    loading: Res<Registry<LoadingTextureAtlas>>,
    mut texture_atlases: ResMut<Registry<CosmosTextureAtlas>>,
    mut images: ResMut<Assets<Image>>,
    mut event_writer: EventWriter<AllTexturesDoneLoadingEvent>,
    mut ev_asset_folder_event: EventReader<AssetEvent<LoadedFolder>>,
    loaded_folders: Res<Assets<LoadedFolder>>,
) {
    for ev in ev_asset_folder_event.read() {
        if let AssetEvent::LoadedWithDependencies { id } = ev {
            let asset = server.get_id_handle::<LoadedFolder>(*id).unwrap();

            if let Some(loaded_folder) = loaded_folders.get(asset) {
                // all assets are now ready, construct texture atlas
                // for better performance

                let mut texture_atlas_builder = SquareTextureAtlasBuilder::new(16);

                for handle in loaded_folder.handles.iter() {
                    texture_atlas_builder.add_texture(handle.clone().typed::<Image>());
                }

                let atlas = texture_atlas_builder.create_atlas(&mut images);

                texture_atlases.register(CosmosTextureAtlas::new("cosmos:main", atlas));

                // Clear out handles to avoid continually checking
                commands.remove_resource::<Registry<LoadingTextureAtlas>>();

                // (note: if you don't have any other handles to the assets
                // elsewhere, they will get unloaded after this)

                event_writer.send(AllTexturesDoneLoadingEvent);
            }
        }
    }

    // let mut handles = Vec::new();
    for folder_handle in loading.iter().map(|h| &h.handles) {
        let load_state = server.get_load_state(folder_handle);
        if load_state == Some(LoadState::Loaded) || load_state == Some(LoadState::Failed) {
            match server.get_recursive_dependency_load_state(folder_handle) {
                Some(RecursiveDependencyLoadState::Loaded) => {}
                Some(RecursiveDependencyLoadState::Failed) => {
                    panic!("Failed to load asset!!");
                }
                _ => {
                    // NotLoaded/Loading: not fully ready yet
                }
            }
        }
    }
}

// #[derive(Debug, Clone)]
// /// Contains information that links the block faces to their texture indices.
// ///
// /// This could also link non-face imformation to their texture indices.
// // struct BlockTextureIndicies(HashMap<String, u32>);
// struct BlockTextureIndicies(LoadedTexture);

// impl BlockTextureIndicies {
//     fn all(index: u32) -> Self {
//         let mut map = HashMap::new();
//         map.insert("all".into(), index);
//         Self(map)
//     }

//     fn new(map: HashMap<String, u32>) -> Self {
//         Self(map)
//     }
// }

#[derive(Debug, Clone)]
/// Links blocks to their correspoding atlas index.
pub struct BlockTextureIndex {
    lod_texture: Option<LoadedTextureType>,
    indices: LoadedTexture,
    id: u16,
    unlocalized_name: String,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BlockNeighbors: usize {
        const Left = 0b1;
        const Right = 0b10;
        const Top = 0b100;
        const Bottom = 0b1000;
    }
}

impl BlockTextureIndex {
    #[inline]
    /// Returns the index for that block face, if one exists
    pub fn atlas_index_from_face(&self, face: BlockFace, neighbors: BlockNeighbors) -> Option<u32> {
        match &self.indices {
            LoadedTexture::All(texture_type) => get_texture_index_from_type(texture_type, neighbors),
            LoadedTexture::Sides {
                right,
                left,
                top,
                bottom,
                front,
                back,
            } => match face {
                BlockFace::Right => get_texture_index_from_type(right, neighbors),
                BlockFace::Left => get_texture_index_from_type(left, neighbors),
                BlockFace::Top => get_texture_index_from_type(top, neighbors),
                BlockFace::Bottom => get_texture_index_from_type(bottom, neighbors),
                BlockFace::Front => get_texture_index_from_type(front, neighbors),
                BlockFace::Back => get_texture_index_from_type(back, neighbors),
            },
        }
    }

    /// Returns the atlas information for a simplified LOD texture
    pub fn atlas_index_for_lod(&self, neighbors: BlockNeighbors) -> Option<u32> {
        match &self.lod_texture {
            Some(texture_type) => get_texture_index_from_type(texture_type, neighbors),
            None => None,
        }
    }
}

#[inline(always)]
fn get_texture_index_from_type(texture_type: &LoadedTextureType, neighbors: BlockNeighbors) -> Option<u32> {
    match texture_type {
        LoadedTextureType::Single(index) => Some(*index),
        LoadedTextureType::Connected(connected) => Some(connected[neighbors.bits()]),
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

#[derive(Serialize, Deserialize, Debug, Clone)]
/// The material for this block - if none the default material is assumed.
pub struct MaterialData {
    /// The name of the material
    pub name: String,
    /// This data is sent to the material for its own processing, if it is provided
    pub data: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReadBlockInfo {
    material: Option<MaterialData>,
    lod_texture: Option<LoadingTextureType>,
    texture: Option<LoadingTexture>,
    model: Option<ModelData>,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub enum ModelData {
    All(String),
    Sides {
        name: String,
        right: String,
        left: String,
        top: String,
        bottom: String,
        front: String,
        back: String,
        connected: Option<ConnectedModelData>,
    },
}

impl Default for ModelData {
    fn default() -> Self {
        Self::All("cosmos:base_block".into())
    }
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct ConnectedModelData {
    pub right: String,
    pub left: String,
    pub top: String,
    pub bottom: String,
    pub front: String,
    pub back: String,
}

#[derive(Debug, Clone)]
/// Every block will have information about how to render it -- even air
pub struct BlockRenderingInfo {
    pub lod_texture: Option<LoadingTextureType>,
    /// This maps textures ids to the various parts of its model.
    pub texture: LoadingTexture,
    /// This is the model id this block has
    pub model: ModelData,
    /// This data is sent to the material for its own processing, if it is provided
    pub material_data: Option<MaterialData>,

    unlocalized_name: String,
    id: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LoadingTexture {
    All(LoadingTextureType),
    Sides {
        right: LoadingTextureType,
        left: LoadingTextureType,
        top: LoadingTextureType,
        bottom: LoadingTextureType,
        front: LoadingTextureType,
        back: LoadingTextureType,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadingTextureType {
    Single(String),
    Connected(Box<[String; 16]>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LoadedTexture {
    All(LoadedTextureType),
    Sides {
        right: LoadedTextureType,
        left: LoadedTextureType,
        top: LoadedTextureType,
        bottom: LoadedTextureType,
        front: LoadedTextureType,
        back: LoadedTextureType,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadedTextureType {
    Single(u32),
    Connected([u32; 16]),
}

impl Identifiable for BlockRenderingInfo {
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

/// Loads al the block rendering information from their json files.
pub fn load_block_rendering_information(
    blocks: Res<Registry<Block>>,
    atlas_registry: Res<Registry<CosmosTextureAtlas>>,
    server: Res<AssetServer>,
    mut registry: ResMut<Registry<BlockTextureIndex>>,
    mut info_registry: ResMut<Registry<BlockRenderingInfo>>,
) {
    let missing_texture_index = atlas_registry
        .from_id("cosmos:main")
        .expect("Missing main atlas!")
        .texture_atlas
        .get_texture_index(
            &server
                .get_handle("cosmos/images/blocks/missing.png")
                .expect("Missing `missing` texture!!!! *world ends*"),
        )
        .expect("Missing `missing` texture index!!! *world double ends*");

    registry.register(BlockTextureIndex {
        id: 0,
        unlocalized_name: "missing".to_owned(),
        lod_texture: None,
        indices: LoadedTexture::All(LoadedTextureType::Single(missing_texture_index)),
    });

    for block in blocks.iter() {
        let unlocalized_name = block.unlocalized_name();
        let mut split = unlocalized_name.split(':');
        let mod_id = split.next().unwrap();
        let block_name = split.next().unwrap_or(unlocalized_name);

        let json_path = format!("assets/{mod_id}/blocks/{block_name}.json");

        let block_info = if let Ok(block_info) = fs::read(&json_path) {
            let read_info = serde_json::from_slice::<ReadBlockInfo>(&block_info)
                .unwrap_or_else(|e| panic!("Error reading json data in {json_path}\nError: \n{e}\n"));

            BlockRenderingInfo {
                id: 0,
                unlocalized_name: block.unlocalized_name().to_owned(),
                model: read_info.model.unwrap_or_default(),
                lod_texture: read_info.lod_texture,
                texture: read_info
                    .texture
                    .unwrap_or_else(|| LoadingTexture::All(LoadingTextureType::Single(unlocalized_name.to_owned()))),
                material_data: read_info.material,
            }
        } else {
            BlockRenderingInfo {
                texture: LoadingTexture::All(LoadingTextureType::Single(unlocalized_name.to_owned())),
                model: ModelData::default(),
                lod_texture: None,
                id: 0,
                unlocalized_name: block.unlocalized_name().to_owned(),
                material_data: None,
            }
        };

        let map = match &block_info.texture {
            LoadingTexture::All(texture) => LoadedTexture::All(process_loading_texture_type(
                texture,
                &atlas_registry,
                &server,
                missing_texture_index,
            )),
            LoadingTexture::Sides {
                right,
                left,
                top,
                bottom,
                front,
                back,
            } => LoadedTexture::Sides {
                right: process_loading_texture_type(right, &atlas_registry, &server, missing_texture_index),
                left: process_loading_texture_type(left, &atlas_registry, &server, missing_texture_index),
                top: process_loading_texture_type(top, &atlas_registry, &server, missing_texture_index),
                bottom: process_loading_texture_type(bottom, &atlas_registry, &server, missing_texture_index),
                front: process_loading_texture_type(front, &atlas_registry, &server, missing_texture_index),
                back: process_loading_texture_type(back, &atlas_registry, &server, missing_texture_index),
            },
        };

        let lod_texture = block_info
            .lod_texture
            .as_ref()
            .map(|x| process_loading_texture_type(x, &atlas_registry, &server, missing_texture_index));

        registry.register(BlockTextureIndex {
            id: 0,
            unlocalized_name: unlocalized_name.to_owned(),
            lod_texture,
            indices: map,
        });

        info_registry.register(block_info);
    }
}

fn process_loading_texture_type(
    texture: &LoadingTextureType,
    atlas_registry: &Registry<CosmosTextureAtlas>,
    server: &AssetServer,
    missing_texture_index: u32,
) -> LoadedTextureType {
    match texture {
        LoadingTextureType::Single(texture_name) => {
            let mut name_split = texture_name.split(':');

            let mod_id = name_split.next().unwrap();
            let name = name_split
                .next()
                .unwrap_or_else(|| panic!("Invalid texture - {texture_name}. Did you forget the 'cosmos:'?"));

            let index = atlas_registry
                .from_id("cosmos:main") // Eventually load this via the block_info file
                .expect("No main atlas")
                .texture_atlas
                .get_texture_index(&server.get_handle(&format!("{mod_id}/images/blocks/{name}.png")).unwrap_or_default())
                .unwrap_or_else(|| {
                    warn!("Could not find texture with ID {mod_id}:{name}");

                    missing_texture_index
                });

            println!("Doing {texture_name:?} = {index}");

            LoadedTextureType::Single(index)
        }
        LoadingTextureType::Connected(textures) => {
            let texture_indices = textures
                .iter()
                .map(|texture_name| {
                    let mut name_split = texture_name.split(':');

                    let mod_id = name_split.next().unwrap();
                    let name = name_split
                        .next()
                        .unwrap_or_else(|| panic!("Invalid texture - {texture_name}. Did you forget the 'cosmos:'?"));

                    atlas_registry
                        .from_id("cosmos:main") // Eventually load this via the block_info file
                        .expect("No main atlas")
                        .texture_atlas
                        .get_texture_index(&server.get_handle(&format!("{mod_id}/images/blocks/{name}.png")).unwrap_or_default())
                        .unwrap_or(missing_texture_index)
                })
                .collect::<Vec<u32>>()
                .try_into()
                .unwrap();

            LoadedTextureType::Connected(texture_indices)
        }
    }
}

pub(super) fn register(app: &mut App) {
    registry::create_registry::<BlockTextureIndex>(app, "cosmos:block_texture_index");
    registry::create_registry::<LoadingTextureAtlas>(app, "cosmos:loading_texture_atlas");
    registry::create_registry::<BlockRenderingInfo>(app, "cosmos:block_rendering_info");
    registry::create_registry::<CosmosTextureAtlas>(app, "cosmos:texture_atlas");

    app.add_event::<AssetsDoneLoadingEvent>()
        .add_event::<AllTexturesDoneLoadingEvent>()
        .add_systems(
            Update,
            (
                check_assets_ready.run_if(resource_exists::<Registry<LoadingTextureAtlas>>),
                assets_done_loading,
            )
                .run_if(in_state(GameState::PostLoading)),
        )
        .add_systems(OnEnter(GameState::PostLoading), setup_textures)
        .add_systems(OnExit(GameState::PostLoading), load_block_rendering_information);
}
