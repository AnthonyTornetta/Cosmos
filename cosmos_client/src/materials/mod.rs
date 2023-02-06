use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    registry::{self, identifiable::Identifiable},
};

pub struct CosmosMaterial {
    handle: Handle<StandardMaterial>,

    id: u16,
    unlocalized_name: String,
}

impl Identifiable for CosmosMaterial {
    fn id(&self) -> u16 {
        self.id
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name()
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }
}

pub(crate) fn register(app: &mut App) {
    registry::multi_registry::create_multi_registry::<Block, CosmosMaterial>(app);
}
