use std::{ffi::OsStr, fs};

use bevy::prelude::*;
use cosmos_core::{item::Item, registry::Registry, state::GameState};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use super::{LootRange, LootTable, LootTableBuilder};

#[derive(Debug, Serialize, Deserialize)]
struct RawLootEntry {
    item: String,
    quantity: (u32, u32),
    weight: u32,
    amount_required: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RawLootTable {
    n_items: (u32, u32),
    loot: Vec<RawLootEntry>,
}

fn load_loot_tables(mut loot_tables: ResMut<Registry<LootTable>>, items: Res<Registry<Item>>) {
    for file in WalkDir::new("assets/cosmos/loot")
        .max_depth(1)
        .into_iter()
        .flatten()
        .filter(|x| x.file_type().is_file())
    {
        let path = file.path();

        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }

        let data = fs::read(path).unwrap_or_else(|e| panic!("Failed to read {path:?}\n{e:?}"));

        let Ok(lt) = serde_json::de::from_slice::<RawLootTable>(&data).map_err(|e| {
            error!("Error parsing {path:?} - {e:?}");
            e
        }) else {
            continue;
        };

        let name = path.file_name().expect("Bad file name").to_str().unwrap().to_owned();
        let name = name[0..name.len() - ".json".len()].to_owned();
        let table_id = format!("cosmos:{name}");

        let mut loot_table = LootTableBuilder::new(&table_id, LootRange::new(lt.n_items.0, lt.n_items.1));
        for entry in lt.loot {
            let Some(item) = items.from_id(&entry.item) else {
                error!("Missing item {} in loot table {table_id} - did you forget cosmos:?", entry.item);
                continue;
            };

            loot_table = loot_table.add_item(
                item,
                entry.weight,
                LootRange::new(entry.quantity.0, entry.quantity.1),
                entry.amount_required,
            );
        }

        let Some(loot_table) = loot_table.build() else {
            error!("Loot table {name} is empty!");
            continue;
        };

        loot_tables.register(loot_table);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnExit(GameState::PostLoading), load_loot_tables);
}
