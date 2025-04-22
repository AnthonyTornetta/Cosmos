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
    block::{Block, block_face::BlockFace},
    blockitems::BlockItems,
    item::Item,
    loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager},
    registry::{self, Registry, identifiable::Identifiable},
    state::GameState,
    structure::chunk::BlockInfo,
};
use serde::{Deserialize, Serialize};

use crate::asset::texture_atlas::SquareTextureAtlasBuilder;

use super::texture_atlas::SquareTextureAtlas;

#[derive(Resource, Debug, Clone)]
struct LoadingTextureAtlas {
    unlocalized_name: String,
    id: u16,
    folder_handle: Vec<Handle<LoadedFolder>>,
    atlas_builders: Vec<SquareTextureAtlasBuilder>,
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
    pub fn new(unlocalized_name: impl Into<String>, handles: Vec<Handle<LoadedFolder>>) -> Self {
        Self {
            folder_handle: handles,
            id: 0,
            unlocalized_name: unlocalized_name.into(),
            atlas_builders: vec![],
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
    let block_image_handles = server.load_folder("cosmos/images/blocks/");
    let item_image_handles = server.load_folder("cosmos/images/items/");

    loading.register(LoadingTextureAtlas::new(
        "cosmos:main",
        vec![block_image_handles, item_image_handles],
    ));

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
    texture_atlases: Vec<SquareTextureAtlas>,
    unlocalized_name: String,
    id: u16,
}

impl CosmosTextureAtlas {
    /// Creates a new Cosmos texture atlas - a newtype wrapper around a bevy `TextureAtlas`
    pub fn new(unlocalized_name: impl Into<String>, atlases: Vec<SquareTextureAtlas>) -> Self {
        Self {
            unlocalized_name: unlocalized_name.into(),
            id: 0,
            texture_atlases: atlases,
        }
    }

    /// Returns all texture atlases for the different texture sizes that need to be accounted for.
    ///
    /// The ordering of this WILL match the `texture_dimensions_index` passed in the [`super::materials::AddMaterialEvent`].
    pub fn texture_atlases(&self) -> impl Iterator<Item = &'_ SquareTextureAtlas> {
        self.texture_atlases.iter()
    }

    /// Returns the square texture atlas given this dimension index.
    ///
    /// This index should correspond to the ordering of [`Self::texture_atlases`], which should be automatically passed to the
    /// material in the [`super::materials::AddMaterialEvent`].
    pub fn get_atlas_for_dimension_index(&self, dimension_index: u32) -> Option<&SquareTextureAtlas> {
        self.texture_atlases.get(dimension_index as usize)
    }

    /// Returns the texture index for this image handle.
    ///
    /// This image handle has to have been loaded first (run this after [`GameState::PostLoading`]).
    pub fn get_texture_index(&self, handle: &Handle<Image>, images: &Assets<Image>) -> Option<TextureIndex> {
        images.get(handle).and_then(|img| {
            let dims = img.width();

            let texture_atlas = self
                .texture_atlases
                .iter()
                .enumerate()
                .find(|(_, x)| x.individual_image_dimensions() == dims);

            texture_atlas.and_then(|(dimension_index, x)| {
                x.get_texture_index(handle).map(|texture_index| TextureIndex {
                    dimension_index: dimension_index as u32,
                    texture_index,
                })
            })
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash, Reflect)]
/// The index for usage with the [`CosmosTextureAtlas`] for this block.
pub struct TextureIndex {
    /// The atlas that should be used for this block's texture dimensions. Pass this to the [`super::materials::AddMaterialEvent`] event.
    pub dimension_index: u32,
    /// The index within the atlas for this block. This should be used after getting the proper atlas via the [`Self::dimension_index`].
    pub texture_index: u32,
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
    mut loading: ResMut<Registry<LoadingTextureAtlas>>,
    mut texture_atlases: ResMut<Registry<CosmosTextureAtlas>>,
    mut images: ResMut<Assets<Image>>,
    mut event_writer: EventWriter<AllTexturesDoneLoadingEvent>,
    mut ev_asset_folder_event: EventReader<AssetEvent<LoadedFolder>>,
    loaded_folders: Res<Assets<LoadedFolder>>,
) {
    for ev in ev_asset_folder_event.read() {
        if let AssetEvent::LoadedWithDependencies { id } = ev {
            let asset = server.get_id_handle::<LoadedFolder>(*id).unwrap();

            if let Some(loaded_folder) = loaded_folders.get(&asset) {
                if let Some(loading_texture_atlases) = loading.iter_mut().find(|x| x.folder_handle.contains(&asset)) {
                    // all assets are now ready, construct texture atlas for better performance

                    // let mut texture_atlas_builder = SquareTextureAtlasBuilder::new(16);

                    for handle in loaded_folder.handles.iter().map(|x| x.clone().typed::<Image>()) {
                        let Some(img) = images.get(&handle) else {
                            continue;
                        };

                        let dims = img.width();

                        if let Some(builder) = loading_texture_atlases
                            .atlas_builders
                            .iter_mut()
                            .find(|x| x.texture_dimensions == dims)
                        {
                            builder.add_texture(handle);
                        } else {
                            let mut builder = SquareTextureAtlasBuilder::new(dims);
                            builder.add_texture(handle);
                            loading_texture_atlases.atlas_builders.push(builder);
                        }
                    }

                    let (idx, _) = loading_texture_atlases
                        .folder_handle
                        .iter()
                        .enumerate()
                        .find(|(_, x)| *x == &asset)
                        .expect("Guarenteed above");

                    loading_texture_atlases.folder_handle.remove(idx);

                    if loading_texture_atlases.folder_handle.is_empty() {
                        let loading_atlas = std::mem::replace(
                            loading_texture_atlases,
                            LoadingTextureAtlas {
                                atlas_builders: vec![],
                                folder_handle: vec![],
                                id: loading_texture_atlases.id,
                                unlocalized_name: loading_texture_atlases.unlocalized_name.to_owned(),
                            },
                        );

                        texture_atlases.register(CosmosTextureAtlas::new(
                            loading_atlas.unlocalized_name,
                            loading_atlas
                                .atlas_builders
                                .into_iter()
                                .map(|id| id.create_atlas(&mut images))
                                .collect(),
                        ));

                        // Clear out handles to avoid continually checking
                        commands.remove_resource::<Registry<LoadingTextureAtlas>>();

                        info!("Sending all textures done loading event!");
                        event_writer.send(AllTexturesDoneLoadingEvent);
                    }
                }
            }
        }
    }

    for folder_handle in loading.iter().map(|h| &h.folder_handle) {
        for handle in folder_handle {
            let load_state = server.get_load_state(handle);
            if matches!(load_state, Some(LoadState::Failed(_) | LoadState::Loaded)) {
                match server.get_recursive_dependency_load_state(handle) {
                    Some(RecursiveDependencyLoadState::Loaded) => {}
                    Some(RecursiveDependencyLoadState::Failed(_)) => {
                        panic!("Failed to load asset!!");
                    }
                    _ => {
                        // NotLoaded/Loading: not fully ready yet
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
/// Links blocks to their correspoding atlas index.
pub struct BlockTextureIndex {
    lod_texture: Option<LoadedTextureType>,
    texture: LoadedTexture,
    id: u16,
    unlocalized_name: String,
}

#[derive(Debug, Clone)]
/// Links items to their correspoding atlas index.
pub struct ItemTextureIndex {
    /// The item's texture index
    texture: TextureIndex,
    id: u16,
    unlocalized_name: String,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    /// Flags that control connected textures
    ///
    /// If this is a part of a structure, you should compute the blocks that are in these positions
    /// relative to the face.
    pub struct BlockNeighbors: usize {
        /// There is a block this should connect with to the left of this face
        const Left = 0b1;
        /// There is a block this should connect with to the right of this face
        const Right = 0b10;
        /// There is a block this should connect with to the top of this face
        const Top = 0b100;
        /// There is a block this should connect with to the bottom of this face
        const Bottom = 0b1000;
    }
}

impl BlockTextureIndex {
    #[inline]
    /// Returns the index for that block face, if one exists
    pub fn atlas_index_from_face(&self, face: BlockFace, neighbors: BlockNeighbors, block_data: BlockInfo) -> TextureIndex {
        match &self.texture {
            LoadedTexture::All(texture_type) => get_texture_index_from_type_and_data(texture_type, neighbors, block_data),
            LoadedTexture::Sides(sides) => match face {
                BlockFace::Right => get_texture_index_from_type_and_data(&sides.right, neighbors, block_data),
                BlockFace::Left => get_texture_index_from_type_and_data(&sides.left, neighbors, block_data),
                BlockFace::Top => get_texture_index_from_type_and_data(&sides.top, neighbors, block_data),
                BlockFace::Bottom => get_texture_index_from_type_and_data(&sides.bottom, neighbors, block_data),
                BlockFace::Front => get_texture_index_from_type_and_data(&sides.front, neighbors, block_data),
                BlockFace::Back => get_texture_index_from_type_and_data(&sides.back, neighbors, block_data),
            },
        }
    }

    /// Returns the atlas information for a simplified LOD texture
    pub fn atlas_index_for_lod(&self, neighbors: BlockNeighbors) -> Option<TextureIndex> {
        match &self.lod_texture {
            Some(texture_type) => Some(get_texture_index_from_type_and_data(texture_type, neighbors, BlockInfo::default())),
            None => None,
        }
    }
}

#[inline(always)]
fn get_texture_index_from_type_and_data(texture_type: &LoadedTextureType, neighbors: BlockNeighbors, data: BlockInfo) -> TextureIndex {
    match texture_type {
        LoadedTextureType::Single(selector) => handle_select_texture(selector, data),
        LoadedTextureType::Connected(connected) => {
            let checking = &connected[neighbors.bits()];
            handle_select_texture(checking, data)
        }
    }
}

fn handle_select_texture(selector: &TextureSelector, data: BlockInfo) -> TextureIndex {
    match selector {
        TextureSelector::Normal(index) => *index,
        TextureSelector::DataDriven(dd) => handle_data_driven(data, dd),
    }
}

fn handle_data_driven(data: BlockInfo, dd: &Box<DataDrivenTextureIndex>) -> TextureIndex {
    for bit in (0..8).rev() {
        if data.0 & (1 << bit) != 0 {
            if let Some(idx) = dd.bit_textures[bit] {
                return idx;
            }
        }
    }

    dd.default
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

impl ItemTextureIndex {
    #[inline]
    /// Returns the selector for that item
    pub fn atlas_index(&self) -> &TextureIndex {
        &self.texture
    }
}

impl Identifiable for ItemTextureIndex {
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

#[derive(Serialize, Deserialize, Debug)]
struct ReadItemInfo {
    material: Option<MaterialData>,
    texture: Option<String>,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
/// The block is made up of models that are divided into separate faces.
pub struct SideRenderingInfo {
    /// The model's name. This should almost always be the unlocalized_name of the block it belongs to.
    ///
    /// This should be unique to the combination of faces & connected faces.
    pub name: String,
    /// The model for the right block face
    pub right: String,
    /// The model for the left block face
    pub left: String,
    /// The model for the top block face
    pub top: String,
    /// The model for the bottom block face
    pub bottom: String,
    /// The model for the front block face
    pub front: String,
    /// The model for the back block face
    pub back: String,
    /// If this should have separate faces used when adjacent to other types of itself, this field can be used
    pub connected: Option<ConnectedModelData>,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
/// Points to the model files of this block
pub enum ModelData {
    /// The block is made up of one model and cannot be divided into separate faces.
    All(String),
    /// The block is made up of models that are divided into separate faces.
    Sides(Box<SideRenderingInfo>),
}

impl Default for ModelData {
    fn default() -> Self {
        Self::All("cosmos:base_block".into())
    }
}

#[derive(Serialize, Clone, Deserialize, Debug)]
/// These models whill be used when the same type of block is placed adjacent to one of the faces of this block
pub struct ConnectedModelData {
    /// Used when the same type of block is placed adjacent to the right face. This will replace the normal right face model.
    pub right: String,
    /// Used when the same type of block is placed adjacent to the left face. This will replace the normal left face model.
    pub left: String,
    /// Used when the same type of block is placed adjacent to the top face. This will replace the normal top face model.
    pub top: String,
    /// Used when the same type of block is placed adjacent to the bottom face. This will replace the normal bottom face model.
    pub bottom: String,
    /// Used when the same type of block is placed adjacent to the front face. This will replace the normal front face model.
    pub front: String,
    /// Used when the same type of block is placed adjacent to the back face. This will replace the normal back face model.
    pub back: String,
}

#[derive(Debug, Clone)]
/// Every block will have information about how to render it -- even air
pub struct BlockRenderingInfo {
    /// Texture used when rendering LODs
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

#[derive(Debug, Clone)]
/// Every block will have information about how to render it -- even air
pub struct ItemRenderingInfo {
    /// This maps textures ids to the various parts of its model.
    pub texture: String,
    // /// This is the model id this block has
    // pub model: ModelData,
    /// This data is sent to the material for its own processing, if it is provided
    pub material_data: Option<MaterialData>,

    unlocalized_name: String,
    id: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Indicates the texture that should be used for this block.
///
/// This is for textures that haven't been assigned indexes yet.
///
/// This will get turned into a [`LoadedTexture`] during the loading phase.
pub enum LoadingTexture {
    /// Each side uses the same texture
    All(LoadingTextureType),
    /// Each side uses a different texture
    Sides {
        /// The right face's texture
        right: LoadingTextureType,
        /// The left face's texture
        left: LoadingTextureType,
        /// The top face's texture
        top: LoadingTextureType,
        /// The bottom face's texture
        bottom: LoadingTextureType,
        /// The front face's texture
        front: LoadingTextureType,
        /// The back face's texture
        back: LoadingTextureType,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Indicates if this texture is connected or is single
pub enum LoadingTextureType {
    /// This texture will not respond to nearby blocks
    Single(String),
    /// This texture will change based on nearby blocks.
    ///
    /// Index order is based on the bitwise value of [`BlockNeighbor`].
    /// Check the docs for how you should set these textures.
    /// TODO: make docs. For now just check out how glass works.
    Connected(Box<[String; 16]>),
    DataDriven(Box<LoadingDataDrivenTextureType>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingDataDrivenTextureType {
    bit_textures: [Option<String>; 8],
    default: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Each side uses a different texture
pub struct LoadedTextureSides {
    /// The right face's texture
    right: LoadedTextureType,
    /// The left face's texture
    left: LoadedTextureType,
    /// The top face's texture
    top: LoadedTextureType,
    /// The bottom face's texture
    bottom: LoadedTextureType,
    /// The front face's texture
    front: LoadedTextureType,
    /// The back face's texture
    back: LoadedTextureType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Indicates the texture that should be used for this block.
pub enum LoadedTexture {
    /// Each side uses the same texture
    All(LoadedTextureType),
    /// Each side uses a different texture
    Sides(Box<LoadedTextureSides>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Indicates if this texture is connected or is single
pub enum LoadedTextureType {
    /// This texture will not respond to nearby blocks
    Single(TextureSelector),
    /// This texture will change based on nearby blocks.
    ///
    /// Index order is based on the bitwise value of [`BlockNeighbors`].
    /// Check the docs for how you should set these textures.
    /// TODO: make docs. For now just check out how glass works.
    Connected([TextureSelector; 16]),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextureSelector {
    Normal(TextureIndex),
    DataDriven(Box<DataDrivenTextureIndex>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDrivenTextureIndex {
    bit_textures: [Option<TextureIndex>; 8],
    default: TextureIndex,
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

impl Identifiable for ItemRenderingInfo {
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
    images: Res<Assets<Image>>,
    mut registry: ResMut<Registry<BlockTextureIndex>>,
    mut info_registry: ResMut<Registry<BlockRenderingInfo>>,
) {
    info!("Loading block JSON files!");

    let missing_texture_index = atlas_registry
        .from_id("cosmos:main")
        .expect("Missing main atlas!")
        .get_texture_index(
            &server
                .get_handle("cosmos/images/blocks/missing.png")
                .expect("Missing `missing` texture!!!! *world ends*"),
            &images,
        )
        .expect("Missing `missing` texture index!!! *world double ends*");

    registry.register(BlockTextureIndex {
        id: 0,
        unlocalized_name: "missing".to_owned(),
        lod_texture: None,
        texture: LoadedTexture::All(LoadedTextureType::Single(TextureSelector::Normal(missing_texture_index))),
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
                &images,
                missing_texture_index,
                "blocks",
            )),
            LoadingTexture::Sides {
                right,
                left,
                top,
                bottom,
                front,
                back,
            } => LoadedTexture::Sides(Box::new(LoadedTextureSides {
                right: process_loading_texture_type(right, &atlas_registry, &server, &images, missing_texture_index, "blocks"),
                left: process_loading_texture_type(left, &atlas_registry, &server, &images, missing_texture_index, "blocks"),
                top: process_loading_texture_type(top, &atlas_registry, &server, &images, missing_texture_index, "blocks"),
                bottom: process_loading_texture_type(bottom, &atlas_registry, &server, &images, missing_texture_index, "blocks"),
                front: process_loading_texture_type(front, &atlas_registry, &server, &images, missing_texture_index, "blocks"),
                back: process_loading_texture_type(back, &atlas_registry, &server, &images, missing_texture_index, "blocks"),
            })),
        };

        let lod_texture = block_info
            .lod_texture
            .as_ref()
            .map(|x| process_loading_texture_type(x, &atlas_registry, &server, &images, missing_texture_index, "blocks"));

        registry.register(BlockTextureIndex {
            id: 0,
            unlocalized_name: unlocalized_name.to_owned(),
            lod_texture,
            texture: map,
        });

        info_registry.register(block_info);
    }
}

/// Loads al the block rendering information from their json files.
fn load_item_rendering_information(
    items: Res<Registry<Item>>,
    atlas_registry: Res<Registry<CosmosTextureAtlas>>,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut registry: ResMut<Registry<ItemTextureIndex>>,
    mut info_registry: ResMut<Registry<ItemRenderingInfo>>,
    block_items: Res<BlockItems>,
) {
    info!("Loading item JSON files!");

    let missing_texture_index = atlas_registry
        .from_id("cosmos:main")
        .expect("Missing main atlas!")
        .get_texture_index(
            &server
                .get_handle("cosmos/images/items/missing.png")
                .expect("Missing item `missing` texture!!!! *world ends*"),
            &images,
        )
        .expect("Missing item `missing` texture index!!! *world double ends*");

    registry.register(ItemTextureIndex {
        id: 0,
        unlocalized_name: "missing".to_owned(),
        texture: missing_texture_index,
    });

    for item in items.iter() {
        if block_items.block_from_item(item).is_some() {
            // No need to create rendering info for items that will be rendered as blocks.
            continue;
        }

        let unlocalized_name = item.unlocalized_name();
        let mut split = unlocalized_name.split(':');
        let mod_id = split.next().unwrap();
        let item_name = split.next().unwrap_or(unlocalized_name);

        let json_path = format!("assets/{mod_id}/items/{item_name}.json");

        let item_info = if let Ok(block_info) = fs::read(&json_path) {
            let read_info = serde_json::from_slice::<ReadItemInfo>(&block_info)
                .unwrap_or_else(|e| panic!("Error reading json data in {json_path}\nError: \n{e}\n"));

            ItemRenderingInfo {
                id: 0,
                unlocalized_name: item.unlocalized_name().to_owned(),
                texture: read_info.texture.unwrap_or_else(|| unlocalized_name.to_owned()),
                material_data: read_info.material,
            }
        } else {
            ItemRenderingInfo {
                texture: unlocalized_name.to_owned(),
                id: 0,
                unlocalized_name: item.unlocalized_name().to_owned(),
                material_data: None,
            }
        };

        let map = process_loading_texture_type(
            &LoadingTextureType::Single(item_info.texture.clone()),
            &atlas_registry,
            &server,
            &images,
            missing_texture_index,
            "items",
        );

        // Item's don't support different block face textures.
        let LoadedTextureType::Single(texture) = map else { unreachable!() };

        let item_texture_index = match texture {
            TextureSelector::Normal(id) => id,
            TextureSelector::DataDriven(dd) => dd.default,
        };

        registry.register(ItemTextureIndex {
            id: 0,
            unlocalized_name: unlocalized_name.to_owned(),
            texture: item_texture_index,
        });

        info_registry.register(item_info);
    }
}

fn process_loading_texture_type(
    texture: &LoadingTextureType,
    atlas_registry: &Registry<CosmosTextureAtlas>,
    server: &AssetServer,
    images: &Assets<Image>,
    missing_texture_index: TextureIndex,
    folder_name: &str,
) -> LoadedTextureType {
    match texture {
        LoadingTextureType::Single(texture_name) => {
            let index = get_texture_index_for_name(atlas_registry, server, images, missing_texture_index, texture_name, folder_name);

            LoadedTextureType::Single(TextureSelector::Normal(index))
        }
        LoadingTextureType::Connected(textures) => {
            let texture_indices = textures
                .iter()
                .map(|texture_name| {
                    get_texture_index_for_name(atlas_registry, server, images, missing_texture_index, texture_name, folder_name)
                })
                .map(|x| TextureSelector::Normal(x))
                .collect::<Vec<TextureSelector>>()
                .try_into()
                .unwrap();

            LoadedTextureType::Connected(texture_indices)
        }
        LoadingTextureType::DataDriven(data_driven) => {
            let selector =
                get_data_driven_texture_selector(atlas_registry, server, images, missing_texture_index, folder_name, data_driven);

            LoadedTextureType::Single(selector)
        }
    }
}

fn get_data_driven_texture_selector(
    atlas_registry: &Registry<CosmosTextureAtlas>,
    server: &AssetServer,
    images: &Assets<Image>,
    missing_texture_index: TextureIndex,
    folder_name: &str,
    data_driven: &Box<LoadingDataDrivenTextureType>,
) -> TextureSelector {
    let default = get_texture_index_for_name(
        atlas_registry,
        server,
        images,
        missing_texture_index,
        &data_driven.default,
        folder_name,
    );

    let bit_textures = data_driven
        .bit_textures
        .iter()
        .map(|dd_texture| {
            dd_texture
                .as_ref()
                .map(|name| get_texture_index_for_name(atlas_registry, server, images, missing_texture_index, name, folder_name))
        })
        .collect::<Vec<Option<TextureIndex>>>()
        .try_into()
        .unwrap();

    let selector = TextureSelector::DataDriven(Box::new(DataDrivenTextureIndex { default, bit_textures }));
    selector
}

fn get_texture_index_for_name(
    atlas_registry: &Registry<CosmosTextureAtlas>,
    server: &AssetServer,
    images: &Assets<Image>,
    missing_texture_index: TextureIndex,
    texture_name: &String,
    folder_name: &str,
) -> TextureIndex {
    let mut name_split = texture_name.split(':');

    let mod_id = name_split.next().unwrap();
    let name = name_split
        .next()
        .unwrap_or_else(|| panic!("Invalid texture - {texture_name}. Did you forget the 'cosmos:'?"));

    let index = atlas_registry
        .from_id("cosmos:main") // Eventually load this via the block_info file
        .expect("No main atlas")
        .get_texture_index(
            &server
                .get_handle(format!("{mod_id}/images/{folder_name}/{name}.png"))
                .unwrap_or_default(),
            images,
        )
        .unwrap_or_else(|| {
            warn!("Could not find texture with ID {mod_id}:{name}");

            missing_texture_index
        });

    index
}

/// This is to resolve ambiguity issues. Because ambiguity detection can't detect
/// different state run conditions, this set is used to let bevy understand they're different
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum AssetsSet {
    /// Run in [`GameState::PostLoading`] to load assets.
    AssetsLoading,
    /// Run in [`GameState::Playing`] to use those loaded assets.
    AssetsReady,
}

/// In PostLoading state.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum ItemMeshingLoadingSet {
    /// Registries [`ItemTextureIndex`] and [`ItemRenderingInfo`] are setup.
    LoadItemRenderingInformation,
    /// The model files for blocks are loaded.
    LoadBlockModels,
    /// The model files for items are loaded.
    LoadItemModels,
    /// Meshes are inserted into the [`ItemRenderingInfo`] registry.
    ///
    /// This MUST happen after both block and item model files are properly loaded.
    GenerateMeshes,
}

pub(super) fn register(app: &mut App) {
    registry::create_registry::<BlockTextureIndex>(app, "cosmos:block_texture_index");
    registry::create_registry::<ItemTextureIndex>(app, "cosmos:item_texture_index");
    registry::create_registry::<LoadingTextureAtlas>(app, "cosmos:loading_texture_atlas");
    registry::create_registry::<BlockRenderingInfo>(app, "cosmos:block_rendering_info");
    registry::create_registry::<ItemRenderingInfo>(app, "cosmos:item_rendering_info");
    registry::create_registry::<CosmosTextureAtlas>(app, "cosmos:texture_atlas");

    app.configure_sets(Update, (AssetsSet::AssetsLoading, AssetsSet::AssetsReady).chain());

    app.configure_sets(
        OnExit(GameState::PostLoading),
        (
            ItemMeshingLoadingSet::LoadItemRenderingInformation,
            ItemMeshingLoadingSet::LoadBlockModels,
            ItemMeshingLoadingSet::LoadItemModels,
            ItemMeshingLoadingSet::GenerateMeshes,
        )
            .chain(),
    );

    app.add_event::<AssetsDoneLoadingEvent>()
        .add_event::<AllTexturesDoneLoadingEvent>()
        .add_systems(
            Update,
            (
                check_assets_ready.run_if(resource_exists::<Registry<LoadingTextureAtlas>>),
                assets_done_loading,
            )
                .in_set(AssetsSet::AssetsLoading)
                .chain()
                .run_if(in_state(GameState::PostLoading)),
        )
        .add_systems(OnEnter(GameState::PostLoading), setup_textures)
        .add_systems(
            OnExit(GameState::PostLoading),
            (load_item_rendering_information, load_block_rendering_information)
                .in_set(ItemMeshingLoadingSet::LoadItemRenderingInformation)
                .chain(),
        );
}
