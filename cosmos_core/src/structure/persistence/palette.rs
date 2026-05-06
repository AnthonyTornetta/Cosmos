use bevy::{platform::collections::HashMap, prelude::*};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    block::Block,
    netty::sync::IdentifiableComponent,
    prelude::Structure,
    registry::{Registry, identifiable::Identifiable},
};

#[derive(Serialize, Deserialize, Reflect, Clone, Debug, Component)]
pub struct Palette(HashMap<u16, String>);

impl IdentifiableComponent for Palette {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:palette"
    }
}

impl Palette {
    pub fn new_from_structure(structure: &Structure, blocks: &Registry<Block>) -> Self {
        Palette(
            structure
                .all_blocks_iter(false)
                .map(|c| structure.block_id_at(c))
                .unique()
                .map(|x| (x, blocks.from_numeric_id(x).unlocalized_name().to_owned()))
                .collect::<HashMap<u16, String>>(),
        )
    }

    pub fn exists(&self, id: u16) -> bool {
        self.0.contains_key(&id)
    }

    pub fn get(&self, id: u16) -> Option<&String> {
        self.0.get(&id)
    }

    pub fn set(&mut self, id: u16, str_id: String) {
        self.0.insert(id, str_id);
    }
}
