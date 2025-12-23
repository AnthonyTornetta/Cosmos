//! Advanced build mode

use std::collections::HashSet;

use bevy::prelude::*;
use cosmos_core::{
    netty::client::LocalPlayer,
    prelude::{BlockCoordinate, Structure, UnboundBlockCoordinate},
    structure::shared::build_mode::BuildMode,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    interactions::block_interactions::{LookedAtBlock, LookingAt},
};

#[derive(Component)]
struct AdvancedBuild;

#[derive(Component, Clone, Copy, Debug, Reflect, Default)]
enum AdvancedBuildMode {
    #[default]
    Area,
}

fn compute_area_blocks(looking_at: LookedAtBlock, structure: &Structure) -> Vec<BlockCoordinate> {
    if !structure.has_block_at(looking_at.block.coords()) {
        info!("no block at ;(");
        return vec![];
    }

    let Ok(start_search) = BlockCoordinate::try_from(looking_at.block.coords() + looking_at.block_dir.to_coordinates()) else {
        info!("bad ss;(");
        return vec![];
    };

    if !structure.is_within_blocks(start_search) {
        info!("1;(");
        return vec![];
    }

    if !structure.has_block_at(looking_at.block.coords()) {
        info!("2;(");
        return vec![];
    }

    if structure.has_block_at(start_search) {
        info!("3;(");
        return vec![];
    }

    let mut all_blocks = vec![];

    const MAX_SEARCH_N: usize = 100;
    let mut done = HashSet::new();
    let mut to_search = HashSet::new();
    to_search.insert(start_search);

    let dirs = looking_at.block_dir.other_axes_and_inverse();

    while !to_search.is_empty() {
        let mut next_todo = HashSet::default();

        for &search in &to_search {
            done.insert(search);
            all_blocks.push(search);

            if all_blocks.len() > MAX_SEARCH_N {
                return all_blocks;
            }

            for &dir in &dirs {
                let Ok(next_search) = BlockCoordinate::try_from(search + dir.to_coordinates()) else {
                    continue;
                };

                if !structure.is_within_blocks(next_search) {
                    continue;
                }

                if structure.has_block_at(next_search) {
                    continue;
                }

                let Ok(below) =
                    BlockCoordinate::try_from(UnboundBlockCoordinate::from(next_search) - looking_at.block_dir.to_coordinates())
                else {
                    continue;
                };

                if !structure.has_block_at(below) {
                    continue;
                }

                if done.contains(&next_search) {
                    continue;
                }

                next_todo.insert(next_search);
            }
        }

        to_search = next_todo;
    }

    all_blocks
}

impl AdvancedBuildMode {
    fn compute_blocks_on_place(&self, looking_at: LookedAtBlock, structure: &Structure) -> Vec<BlockCoordinate> {
        match *self {
            Self::Area => compute_area_blocks(looking_at, structure),
        }
    }
}

fn toggle_advanced_build(
    mut commands: Commands,
    inputs: InputChecker,
    q_player: Query<(Entity, Has<AdvancedBuild>), (With<LocalPlayer>, With<BuildMode>)>,
) {
    if !inputs.check_just_pressed(CosmosInputs::AdvancedBuildModeToggle) {
        return;
    }

    let Ok((ent, is_adv)) = q_player.single() else {
        return;
    };

    if is_adv {
        info!("Removed adv.");
        commands.entity(ent).remove::<AdvancedBuild>();
    } else {
        info!("Enabled adv.");
        commands.entity(ent).insert(AdvancedBuild);
    }
}

fn render_advanced_build_mode(
    q_structure: Query<&Structure>,
    q_mode: Query<(&LookingAt, Option<&AdvancedBuildMode>), (With<LocalPlayer>, With<AdvancedBuild>)>,
) {
    let Ok((looking_at, mode)) = q_mode.single() else {
        info!("not looking");
        return;
    };

    let mode = mode.copied().unwrap_or_default();

    let Some(block) = looking_at.looking_at_block else {
        info!("not looking @ block");
        return;
    };

    let Ok(structure) = q_structure.get(block.block.structure()) else {
        info!("bad struct");
        return;
    };

    let blocks = mode.compute_blocks_on_place(block, structure);

    info!("{blocks:?}");
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (toggle_advanced_build, render_advanced_build_mode).chain());
}
