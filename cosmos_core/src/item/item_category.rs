use bevy::{prelude::*, utils::HashSet};
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::registry::sync_registry,
    registry::{create_registry, identifiable::Identifiable},
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ItemCategory {
    unlocalized_name: String,
    item_icon_id: String,
    id: u16,

    items: HashSet<u16>,
}

impl ItemCategory {
    pub fn new(unlocalized_name: impl Into<String>, item_icon_id: impl Into<String>) -> Self {
        Self {
            unlocalized_name: unlocalized_name.into(),
            item_icon_id: item_icon_id.into(),
            items: Default::default(),
            id: 0,
        }
    }
}

impl Identifiable for ItemCategory {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

pub(super) fn register(app: &mut App) {
    create_registry::<ItemCategory>(app, "cosmos:item_category");
    sync_registry::<ItemCategory>(app);
}
