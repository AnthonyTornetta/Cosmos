use std::{ffi::OsStr, fs};

use bevy::prelude::*;
use cosmos_core::{block::Block, item::Item, registry::Registry, state::GameState};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use super::BlockDrops;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RawItemDropEntry {
    item: String,
    weight: f32,
    quantity: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RawDrops {
    block: String,
    drops: Vec<RawItemDropEntry>,
}

fn load_drop_jsons(blocks: Res<Registry<Block>>, items: Res<Registry<Item>>, mut block_drops: ResMut<BlockDrops>) {
    'dir_loop: for entry in WalkDir::new("assets/cosmos/drops/block").max_depth(1) {
        let Ok(entry) = entry else {
            continue;
        };

        let path = entry.path();
        if path.is_dir() || path.extension().and_then(OsStr::to_str) != Some("json") {
            continue;
        }

        let drop_json = fs::read(path).unwrap_or_else(|e| panic!("Unable to read drop file {path:?}\n{e:?}"));

        let recipe = serde_json::from_slice::<RawDrops>(&drop_json).unwrap_or_else(|e| panic!("Invalid recipe json {path:?}\n{e:?}"));

        let Some(block) = blocks.from_id(&recipe.block) else {
            error!("Error loading recipe {path:?} - unable to find block {}", recipe.block);
            continue;
        };

        for entry in recipe.drops {
            let Some(item) = items.from_id(&entry.item) else {
                error!("Error loading recipe {path:?} - unable to find block {}", recipe.block);
                continue 'dir_loop;
            };

            block_drops.add_drop(block, item, entry.weight, entry.quantity);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), load_drop_jsons);
}
