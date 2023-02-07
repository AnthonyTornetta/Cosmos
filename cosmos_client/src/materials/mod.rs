use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    registry::{self, identifiable::Identifiable, multi_registry::MultiRegistry, Registry},
};

use crate::{
    asset::asset_loading::{IlluminatedAtlas, MainAtlas},
    state::game_state::GameState,
};

pub struct CosmosMaterial {
    pub handle: Handle<StandardMaterial>,

    id: u16,
    unlocalized_name: String,
}

impl CosmosMaterial {
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
    mut registry: ResMut<MultiRegistry<Block, CosmosMaterial>>,
    main_atlas: Res<MainAtlas>,
    illum_atlas: Res<IlluminatedAtlas>,
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
        if block.unlocalized_name() != "cosmos:light" {
            registry
                .add_link(block, "cosmos:main")
                .expect("Main material should exist");
        }
    }

    if let Some(light) = blocks.from_id("cosmos:light") {
        registry
            .add_link(light, "cosmos:illuminated")
            .expect("Illuminated material should exist");
    }
}

pub(crate) fn register(app: &mut App) {
    registry::multi_registry::create_multi_registry::<Block, CosmosMaterial>(app);

    app.add_system_set(SystemSet::on_exit(GameState::PostLoading).with_system(register_materials));
}
