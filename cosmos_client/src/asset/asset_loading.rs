//! Handles the loading of all texture assets.
//!
//! This also combines the textures into one big atlas.

use std::fs;

use bevy::{
    asset::{LoadState, LoadedFolder, RecursiveDependencyLoadState},
    prelude::*,
    utils::HashMap,
};
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

#[derive(Debug, Clone)]
/// Contains information that links the block faces to their texture indices.
///
/// This could also link non-face imformation to their texture indices.
struct BlockTextureIndicies(HashMap<String, u32>);

impl BlockTextureIndicies {
    fn all(index: u32) -> Self {
        let mut map = HashMap::new();
        map.insert("all".into(), index);
        Self(map)
    }

    fn new(map: HashMap<String, u32>) -> Self {
        Self(map)
    }
}

#[derive(Debug, Clone)]
/// Links blocks to their correspoding atlas index.
pub struct BlockTextureIndex {
    indices: BlockTextureIndicies,
    id: u16,
    unlocalized_name: String,
}

impl BlockTextureIndex {
    #[inline]
    /// Returns the index for that block face, if one exists
    pub fn atlas_index_from_face(&self, face: BlockFace) -> Option<u32> {
        self.atlas_index(face.as_str())
    }

    #[inline]
    /// Returns the index for that specific identifier, if one exists.
    ///
    /// If none exists and an "all" identifier is present, "all" is returned.
    pub fn atlas_index(&self, identifier: &str) -> Option<u32> {
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
    texture: Option<HashMap<String, String>>,
    model: Option<String>,
}

#[derive(Debug, Clone)]
/// Every block will have information about how to render it -- even air
pub struct BlockRenderingInfo {
    /// This maps textures ids to the various parts of its model.
    pub texture: HashMap<String, String>,
    /// This is the model id this block has
    pub model: String,
    /// This data is sent to the material for its own processing, if it is provided
    pub material_data: Option<MaterialData>,

    unlocalized_name: String,
    id: u16,
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
    if let Some(index) = atlas_registry
        .from_id("cosmos:main")
        .expect("Missing main atlas!")
        .texture_atlas
        .get_texture_index(
            &server
                .get_handle("cosmos/images/blocks/missing.png")
                .expect("Missing missing texture!!!! *world ends*"),
        )
    {
        registry.register(BlockTextureIndex {
            id: 0,
            unlocalized_name: "missing".to_owned(),
            indices: BlockTextureIndicies::all(index),
        });
    }

    for block in blocks.iter() {
        let unlocalized_name = block.unlocalized_name();
        let mut split = unlocalized_name.split(':');
        let mod_id = split.next().unwrap();
        let block_name = split.next().unwrap_or(unlocalized_name);

        let json_path = format!("assets/{mod_id}/blocks/{block_name}.json");

        let block_info = if let Ok(block_info) = fs::read(&json_path) {
            let read_info = serde_json::from_slice::<ReadBlockInfo>(&block_info)
                .unwrap_or_else(|e| panic!("Error reading json data in {json_path}. \nError: \n{e}\n"));

            BlockRenderingInfo {
                id: 0,
                unlocalized_name: block.unlocalized_name().to_owned(),
                model: read_info.model.unwrap_or("cosmos:base_block".into()),
                texture: read_info.texture.unwrap_or_else(|| {
                    let mut default_hashmap = HashMap::new();
                    default_hashmap.insert("all".into(), unlocalized_name.to_owned());
                    default_hashmap
                }),
                material_data: read_info.material,
            }
        } else {
            let mut default_hashmap = HashMap::new();
            default_hashmap.insert("all".into(), unlocalized_name.to_owned());

            BlockRenderingInfo {
                texture: default_hashmap.clone(),
                model: "cosmos:base_block".into(),
                id: 0,
                unlocalized_name: block.unlocalized_name().to_owned(),
                material_data: None,
            }
        };

        let mut map = HashMap::new();
        for (entry, texture_name) in block_info.texture.iter() {
            let mut name_split = texture_name.split(':');

            let mod_id = name_split.next().unwrap();
            let name = name_split
                .next()
                .unwrap_or_else(|| panic!("Invalid texture - {texture_name}. Did you forget the 'cosmos:'?"));

            if let Some(index) = atlas_registry
                .from_id("cosmos:main") // Eventually load this via the block_info file
                .expect("No main atlas")
                .texture_atlas
                .get_texture_index(&server.get_handle(&format!("{mod_id}/images/blocks/{name}.png")).unwrap_or_default())
            {
                map.insert(entry.to_owned(), index);
            }
        }

        registry.register(BlockTextureIndex {
            id: 0,
            unlocalized_name: unlocalized_name.to_owned(),
            indices: BlockTextureIndicies::new(map),
        });

        info_registry.register(block_info);
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
