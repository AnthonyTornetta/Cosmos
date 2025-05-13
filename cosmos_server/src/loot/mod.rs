//! Randomly generated loot on structures

use bevy::prelude::*;
use cosmos_core::{
    item::Item,
    registry::{Registry, create_registry, identifiable::Identifiable},
};

mod generate;
mod loading;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LootEntry {
    weight: u32,
    item: u16,
    amount: LootRange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LootRange {
    pub low: u32,
    pub high: u32,
}

impl LootRange {
    pub fn new(low: u32, high: u32) -> Self {
        Self {
            low: low.min(high),
            high: low.max(high),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LootTable {
    unlocalized_name: String,
    id: u16,
    n_items: LootRange,
    items: Vec<LootEntry>,
}

impl Identifiable for LootTable {
    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
    fn set_numeric_id(&mut self, id: u16) {
        self.id = id
    }
    fn id(&self) -> u16 {
        self.id
    }
}

impl LootTable {
    pub fn iter(&self) -> impl Iterator<Item = &LootEntry> {
        self.items.iter()
    }

    pub fn total_weight(&self) -> u32 {
        self.items.iter().map(|x| x.weight).sum()
    }
}

pub struct LootTableBuilder(LootTable);

impl LootTableBuilder {
    pub fn new(unlocalized_name: impl Into<String>, items_range: LootRange) -> Self {
        Self(LootTable {
            unlocalized_name: unlocalized_name.into(),
            id: 0,
            n_items: items_range,
            items: vec![],
        })
    }

    pub fn add_item(mut self, item: &Item, weight: u32, amount_range: LootRange) -> Self {
        self.0.items.push(LootEntry {
            item: item.id(),
            amount: amount_range,
            weight,
        });

        self
    }

    pub fn build(self) -> Option<LootTable> {
        if self.0.items.is_empty() { None } else { Some(self.0) }
    }
}

#[derive(Component)]
/// Put this on a structure to automatically fill containers with loot
pub struct NeedsLootGenerated {
    loot_classification: u16,
}

impl NeedsLootGenerated {
    pub fn from_loot_id(loot_id: &str, loot: &Registry<LootTable>) -> Option<Self> {
        loot.from_id(loot_id).map(|x| Self::new(x.id()))
    }

    pub fn new(loot_id: u16) -> Self {
        Self {
            loot_classification: loot_id,
        }
    }
}

pub(super) fn register(app: &mut App) {
    create_registry::<LootTable>(app, "cosmos:loot_tables");

    loading::register(app);
    generate::register(app);
}
