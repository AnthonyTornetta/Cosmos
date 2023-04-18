//! Used to handle material registration

use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    registry::{self, identifiable::Identifiable, many_to_one::ManyToOneRegistry, Registry},
};

use crate::{
    asset::asset_loading::{IlluminatedMaterial, MainAtlas},
    state::game_state::GameState,
};

/// An identifiable `StandardMaterial`
pub struct CosmosMaterial {
    /// The handle to the bevy `StandardMaterial`
    pub handle: Handle<StandardMaterial>,

    id: u16,
    unlocalized_name: String,
}

impl CosmosMaterial {
    /// Creates an identifiable `StandardMaterial`
    pub fn new(unlocalized_name: String, handle: Handle<StandardMaterial>) -> Self {
        Self {
            unlocalized_name,
            handle,
            id: 0,
        }
    }
}

impl Identifiable for CosmosMaterial {
    fn id(&self) -> u16 {
        self.id
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }
}

fn register_materials(
    blocks: Res<Registry<Block>>,
    mut registry: ResMut<ManyToOneRegistry<Block, CosmosMaterial>>,
    main_atlas: Res<MainAtlas>,
    illum_atlas: Res<IlluminatedMaterial>,
) {
    registry.insert_value(CosmosMaterial::new(
        "cosmos:main".to_owned(),
        main_atlas.material.clone(),
    ));

    registry.insert_value(CosmosMaterial::new(
        "cosmos:illuminated".to_owned(),
        illum_atlas.material.clone(),
    ));

    // TODO: Automate this in file or something

    for block in blocks.iter() {
        if block.unlocalized_name() != "cosmos:light"
            && block.unlocalized_name() != "cosmos:ship_core"
        {
            registry
                .add_link(block, "cosmos:main")
                .expect("Main material should exist");
        }
    }

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
}

pub(super) fn register(app: &mut App) {
    registry::many_to_one::create_many_to_one_registry::<Block, CosmosMaterial>(app);

    app.add_system(register_materials.in_schedule(OnExit(GameState::PostLoading)));
}
