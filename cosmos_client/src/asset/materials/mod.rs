//! Used to handle material registration

use std::sync::{Arc, RwLock};

use bevy::{
    prelude::*,
    render::mesh::{MeshVertexAttribute, VertexAttributeValues},
    utils::HashMap,
};
use cosmos_core::{
    block::Block,
    registry::{self, identifiable::Identifiable, many_to_one::ManyToOneRegistry, Registry},
};

use crate::{rendering::MeshInformation, state::game_state::GameState};

use super::asset_loading::{load_block_rendering_information, BlockRenderingInfo};

pub mod animated_material;
pub mod block_materials;
mod compute;
mod compute_2;
pub(super) mod material_types;

#[derive(Hash, Clone, Copy, Debug, PartialEq, Eq)]
/// Materials are used for different things, and sometimes need to be in different states.
///
/// This represents which state the material is used for
pub enum MaterialType {
    /// The normal behavior of your material, like when it's placed on a structure.
    Normal,
    /// Used in the UI and should not respond to any lighting.
    Unlit,
    /// Used in LODs when the blocks are at a certani scale, so if your material should be dumbed down a bit use this.
    ///
    /// For example, all transparent blocks are made opqaue by default.
    FarAway,
}

#[derive(Event)]
/// This event is sent when you have to remove all materials from a given entity.
///
/// If you add your own material, make sure to remove it when you receive this event for your material.
///
/// ### Important:
/// Add your event listeners for this after `remove_materials` and before `add_materials`
pub struct RemoveAllMaterialsEvent {
    /// The entity to remove the materials from
    pub entity: Entity,
}

#[derive(Event)]
/// This event is sent whenever a specific material must be added to an entity.
///
/// ### Important:
/// Add your event listeners for this after `add_materials`.
pub struct AddMaterialEvent {
    /// The entity to add your material to
    pub entity: Entity,
    /// The materal's id referring to the `Registry<MaterialDefinition]`
    pub add_material_id: u16,
    /// The state the material should be in
    pub material_type: MaterialType,
}

/// Add all event listeners for `AddMaterialEvent` after this to prevent any 1-frame delays
pub fn add_materials() {}
/// Add all event listeners for `RemoveAllMaterialsEvent` after this and before `add_materials` to ensure your material is removed at the right time.
pub fn remove_materials() {}

/// Generates any extra information need for meshes that use this material
pub trait MaterialMeshInformationGenerator: Send + Sync {
    /// Generates the information needed for this mesh information.
    ///
    /// It is guarenteeed that this `mesh_info` uses this material.
    fn generate_information(&self, block_id: u16, mesh_info: &MeshInformation) -> Vec<(MeshVertexAttribute, VertexAttributeValues)>;

    /// Adds information about a block from its JSON file if that block uses this material
    fn add_information(&mut self, block_id: u16, additional_information: &HashMap<String, String>);
}

#[derive(Resource, Clone)]
/// This stores the texture atlas for all blocks in the game.
///
/// Eventually this will be redone to allow for multiple atlases, but for now this works fine.
pub struct MaterialDefinition {
    // /// The main material used to draw blocks
    // pub material: Handle<M>,
    // /// The material used to render things far away
    // pub far_away_material: Handle<M>,
    // /// The unlit version of the main material
    // pub unlit_material: Handle<M>,
    id: u16,
    unlocalized_name: String,

    generator: Option<Arc<RwLock<Box<dyn MaterialMeshInformationGenerator>>>>,
}
impl MaterialDefinition {
    /// Creates a new material definition
    pub fn new(unlocalized_name: impl Into<String>, mesh_information_generator: Option<Box<dyn MaterialMeshInformationGenerator>>) -> Self {
        Self {
            id: 0,
            unlocalized_name: unlocalized_name.into(),
            generator: mesh_information_generator.map(|mesh_information_generator| Arc::new(RwLock::new(mesh_information_generator))),
        }
    }

    /// Generates the information needed for this mesh information.
    ///
    /// It is guarenteeed that this `mesh_info` uses this material.
    pub fn add_material_data(&self, block_id: u16, mesh_info: &MeshInformation) -> Vec<(MeshVertexAttribute, VertexAttributeValues)> {
        self.generator
            .as_ref()
            .map(|gen| gen.read().unwrap().generate_information(block_id, mesh_info))
            .unwrap_or_default()
    }

    /// Adds information about a block from its JSON file if that block uses this material
    pub fn add_block_information(&self, block_id: u16, additional_information: &HashMap<String, String>) {
        if let Some(generator) = self.generator.as_ref() {
            generator.write().unwrap().add_information(block_id, additional_information);
        }
    }
}

impl Identifiable for MaterialDefinition {
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

#[derive(Clone, Debug)]
/// Represents a mapping between blocks and the materials they are attached to.
///
/// Do not use the `id` function to get the material's id - that only refers to this mapping's id.
/// Instead use `material_id` to get the material this points to's id.
pub struct BlockMaterialMapping {
    id: u16,
    unlocalized_name: String,
    material_id: u16,
}

impl BlockMaterialMapping {
    /// The id of the material this points to
    pub fn material_id(&self) -> u16 {
        self.material_id
    }
}

impl Identifiable for BlockMaterialMapping {
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

fn register_materials(
    blocks: Res<Registry<Block>>,
    materials: Res<Registry<MaterialDefinition>>,
    mut registry: ResMut<ManyToOneRegistry<Block, BlockMaterialMapping>>,
    info_registry: Res<Registry<BlockRenderingInfo>>,
) {
    for material in materials.iter() {
        registry.insert_value(BlockMaterialMapping {
            id: 0,
            material_id: material.id(),
            unlocalized_name: material.unlocalized_name().to_owned(),
        });
    }

    for (block_name, material_data) in info_registry
        .iter()
        .filter_map(|x| x.material_data.as_ref().map(|y| (x.unlocalized_name(), y)))
    {
        if let Some(block) = blocks.from_id(block_name) {
            let material_name = &material_data.name;

            registry
                .add_link(block, material_name)
                .unwrap_or_else(|_| panic!("Missing material {material_name} for block {block_name}"));

            if let Some(data) = material_data.data.as_ref() {
                materials
                    .from_id(material_name)
                    .expect("This was verified to exist above!")
                    .add_block_information(block.id(), data);
            }
        }
    }

    for block in blocks.iter() {
        if !registry.contains(block) {
            registry.add_link(block, "cosmos:main").expect("Animated material should exist");
        }
    }
}

pub(super) fn register(app: &mut App) {
    registry::create_registry::<MaterialDefinition>(app, "cosmos:material_definitions");
    registry::many_to_one::create_many_to_one_registry::<Block, BlockMaterialMapping>(app);
    material_types::register(app);
    block_materials::register(app);
    animated_material::register(app);
    compute::register(app);
    // compute_2::register(app);
    //
    app.add_systems(
        OnExit(GameState::PostLoading),
        register_materials.after(load_block_rendering_information),
    )
    .add_systems(Update, (remove_materials, add_materials).chain())
    .add_event::<RemoveAllMaterialsEvent>()
    .add_event::<AddMaterialEvent>();
}
