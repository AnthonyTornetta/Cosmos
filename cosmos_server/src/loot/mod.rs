//! Randomly generated loot on structures

use bevy::prelude::*;
use cosmos_core::{
    item::Item,
    registry::{Registry, create_registry, identifiable::Identifiable},
};

mod generate;
mod loading;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A piece of loot that may be generated
pub struct LootEntry {
    weight: u32,
    item: u16,
    amount: LootRange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A range value (low..=high)
pub struct LootRange {
    /// The minimum value
    pub low: u32,
    /// The maximum value
    pub high: u32,
}

impl LootRange {
    /// Assembles a new loot range with these min and max values
    pub fn new(low: u32, high: u32) -> Self {
        Self {
            low: low.min(high),
            high: low.max(high),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Represents all the loot entries that can be generated
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
    /// Iterates over every possible entry
    pub fn iter(&self) -> impl Iterator<Item = &LootEntry> {
        self.items.iter()
    }

    /// Computes the total weight of each entry combined
    pub fn total_weight(&self) -> u32 {
        self.items.iter().map(|x| x.weight).sum()
    }
}

/// Used to easily construct [`LootTable`]s
pub struct LootTableBuilder(LootTable);

impl LootTableBuilder {
    /// Creates a new, empty loot table
    pub fn new(unlocalized_name: impl Into<String>, items_range: LootRange) -> Self {
        Self(LootTable {
            unlocalized_name: unlocalized_name.into(),
            id: 0,
            n_items: items_range,
            items: vec![],
        })
    }

    /// Adds a loot entry to this table
    pub fn add_item(mut self, item: &Item, weight: u32, amount_range: LootRange) -> Self {
        self.0.items.push(LootEntry {
            item: item.id(),
            amount: amount_range,
            weight,
        });

        self
    }

    /// Builds a loot table if any items have been added, otherwise returns [`None`].
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
    /// Returns this component if this is a valid loot registry entry
    pub fn from_loot_id(loot_id: &str, loot: &Registry<LootTable>) -> Option<Self> {
        loot.from_id(loot_id).map(|x| Self::new(x.id()))
    }

    /// Returns this component for this loot id. This assumes the loot id is valid.
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
