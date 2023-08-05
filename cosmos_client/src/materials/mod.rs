//! Used to handle material registration

use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    registry::{self, many_to_one::ManyToOneRegistry, Registry},
};

use crate::{asset::asset_loading::MaterialDefinition, state::game_state::GameState};

fn register_materials(
    blocks: Res<Registry<Block>>,
    materials: Res<Registry<MaterialDefinition>>,
    mut registry: ResMut<ManyToOneRegistry<Block, MaterialDefinition>>,
) {
    for material in materials.iter() {
        registry.insert_value(material.clone())
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

    for block in blocks.iter() {
        if !registry.contains(block) {
            registry.add_link(block, "cosmos:main").expect("Main material should exist");
        }
    }
}

pub(super) fn register(app: &mut App) {
    registry::many_to_one::create_many_to_one_registry::<Block, MaterialDefinition>(app);

    app.add_systems(OnExit(GameState::PostLoading), register_materials);
}
