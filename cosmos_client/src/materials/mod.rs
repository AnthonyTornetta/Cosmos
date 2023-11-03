//! Used to handle material registration

use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    registry::{self, identifiable::Identifiable, many_to_one::ManyToOneRegistry, Registry},
};

use crate::{asset::asset_loading::MaterialDefinition, state::game_state::GameState};

#[derive(Clone)]
pub struct BlockMaterialMapping {
    id: u16,
    unlocalized_name: String,
    material_id: u16,
}

impl BlockMaterialMapping {
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
) {
    for material in materials.iter() {
        registry.insert_value(BlockMaterialMapping {
            id: 0,
            material_id: material.id(),
            unlocalized_name: material.unlocalized_name().to_owned(),
        });
    }

    // TODO: Specify this in file or something

    if let Some(block) = blocks.from_id("cosmos:light") {
        registry
            .add_link(block, "cosmos:illuminated")
            .expect("Illuminated material should exist");
    }

    if let Some(block) = blocks.from_id("cosmos:ship_core") {
        registry
            .add_link(block, "cosmos:illuminated")
            .expect("Illuminated material should exist");
    }

    if let Some(block) = blocks.from_id("cosmos:water") {
        registry
            .add_link(block, "cosmos:transparent")
            .expect("Transparent material should exist");
    }

    if let Some(block) = blocks.from_id("cosmos:ice") {
        registry
            .add_link(block, "cosmos:transparent")
            .expect("Transparent material should exist");
    }

    for block in blocks.iter() {
        if !registry.contains(block) {
            registry.add_link(block, "cosmos:main").expect("Main material should exist");
        }
    }
}

pub(super) fn register(app: &mut App) {
    registry::many_to_one::create_many_to_one_registry::<Block, BlockMaterialMapping>(app);

    app.add_systems(OnExit(GameState::PostLoading), register_materials);
}
