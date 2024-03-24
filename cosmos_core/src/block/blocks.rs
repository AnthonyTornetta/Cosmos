//! Represents all the blocks present in the game.
//!
//! This list is dynamic, and may grow & shrink at any time.
//!
//! The only guarenteed block is air ("cosmos:air").

use crate::block::block_builder::BlockBuilder;
use crate::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};
use crate::registry::{self, Registry};
use bevy::prelude::{App, EventWriter, OnEnter, ResMut, States};

use super::{Block, BlockProperty};

/// Air's ID - this block will always exist
pub static AIR_BLOCK_ID: u16 = 0;

fn add_cosmos_blocks(
    mut blocks: ResMut<Registry<Block>>,
    mut loading: ResMut<LoadingManager>,
    mut end_writer: EventWriter<DoneLoadingEvent>,
    mut start_writer: EventWriter<AddLoadingEvent>,
) {
    let id = loading.register_loader(&mut start_writer);

    blocks.register(
        BlockBuilder::new("cosmos:stone", 10.0, 50.0, 20.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:grass", 3.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:dirt", 3.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:cherry_leaf", 0.1, 1.0, 1.0)
            .add_property(BlockProperty::Transparent)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:redwood_log", 3.0, 30.0, 7.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:redwood_leaf", 0.1, 1.0, 1.0)
            .add_property(BlockProperty::Transparent)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ship_core", 2.0, 20.0, 20.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:energy_cell", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:reactor", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:laser_cannon", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FullyRotatable)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ship_hull_grey", 4.0, 100.0, 10.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:thruster", 2.0, 20.0, 10.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:light", 0.1, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:glass", 4.0, 100.0, 10.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ice".to_owned(), 1.9, 40.0, 10.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:molten_stone", 10.0, 50.0, 10.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:water".to_owned(), 2.0, 50.0, 10.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:cheese", 10.0, 50.0, 10.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(BlockBuilder::new("cosmos:short_grass", 0.1, 1.0, 0.0).create());

    blocks.register(
        BlockBuilder::new("cosmos:sand", 4.0, 10.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:cactus", 0.8, 10.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:build_block", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    // Grey registered above to keep id consistency (move down here in future)
    let ship_hulls = [
        "black",
        "dark_grey",
        "white",
        "blue",
        "dark_blue",
        "brown",
        "green",
        "dark_green",
        "orange",
        "dark_orange",
        "pink",
        "dark_pink",
        "purple",
        "dark_purple",
        "red",
        "dark_red",
        "yellow",
        "dark_yellow",
        "mint",
    ];

    for ship_hull in ship_hulls {
        blocks.register(
            BlockBuilder::new(format!("cosmos:ship_hull_{ship_hull}"), 4.0, 100.0, 10.0)
                .add_property(BlockProperty::Full)
                .create(),
        );
    }

    blocks.register(
        BlockBuilder::new("cosmos:reactor_controller", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FullyRotatable)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:reactor_casing", 2.0, 20.0, 10.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:reactor_window", 2.0, 20.0, 10.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:reactor_cell", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:fan", 2.0, 20.0, 10.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Full)
            .create(),
    );

    let glass_colors = [
        "white",
        "blue",
        "dark_blue",
        "brown",
        "green",
        "dark_green",
        "orange",
        "dark_orange",
        "pink",
        "dark_pink",
        "purple",
        "dark_purple",
        "red",
        "dark_red",
        "yellow",
        "dark_yellow",
        "mint",
    ];

    for color in glass_colors {
        blocks.register(
            BlockBuilder::new(format!("cosmos:glass_{color}"), 4.0, 100.0, 10.0)
                .add_property(BlockProperty::Transparent)
                .add_property(BlockProperty::Full)
                .create(),
        );
    }

    blocks.register(
        BlockBuilder::new("cosmos:storage", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:station_core", 2.0, 20.0, 20.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:test_ore", 10.0, 50.0, 12.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:plasma_drill", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FullyRotatable)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:shop", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:camera", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FullyRotatable)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:gravity_well", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ramp", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::FullyRotatable)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:debug", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::FullyRotatable)
            .add_property(BlockProperty::Full)
            .create(),
    );

    loading.finish_loading(id, &mut end_writer);
}

// Game will break without air & needs this at ID 0
fn add_air_block(
    mut blocks: ResMut<Registry<Block>>,
    mut add_loader_event: EventWriter<AddLoadingEvent>,
    mut done_loading_event: EventWriter<DoneLoadingEvent>,
    mut loader: ResMut<LoadingManager>,
) {
    let id = loader.register_loader(&mut add_loader_event);

    blocks.register(
        BlockBuilder::new("cosmos:air", 0.0, 0.0, 0.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Empty)
            .create(),
    );

    loader.finish_loading(id, &mut done_loading_event);
}

pub(super) fn register<T: States>(app: &mut App, pre_loading_state: T, loading_state: T) {
    registry::create_registry::<Block>(app, "cosmos:blocks");

    app.add_systems(OnEnter(pre_loading_state), add_air_block);
    app.add_systems(OnEnter(loading_state), add_cosmos_blocks);
}
