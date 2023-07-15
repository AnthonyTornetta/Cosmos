//! Represents all the blocks present in the game.
//!
//! This list is dynamic, and may grow & shrink at any time.
//!
//! The only guarenteed block is air ("cosmos:air").

use crate::block::block_builder::BlockBuilder;
use crate::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};
use crate::registry::{self, Registry};
use bevy::prelude::{App, EventWriter, IntoSystemAppConfig, OnEnter, ResMut, States};

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
        BlockBuilder::new("cosmos:stone".into(), 10.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:grass".into(), 3.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:dirt".into(), 3.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:log".into(), 3.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:cherry_leaf".into(), 0.1)
            .add_property(BlockProperty::Transparent)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:redwood_log".into(), 3.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:redwood_leaf".into(), 0.1)
            .add_property(BlockProperty::Transparent)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ship_core".into(), 2.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::ShipOnly)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:energy_cell".to_owned(), 2.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:reactor".to_owned(), 2.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:laser_cannon".to_owned(), 2.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ship_hull".to_owned(), 6.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:thruster".to_owned(), 2.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:light".to_owned(), 0.1)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:glass".to_owned(), 6.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ice".to_owned(), 2.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:water".to_owned(), 6.0)
            .add_property(BlockProperty::Transparent)
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
        BlockBuilder::new("cosmos:air".into(), 0.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Empty)
            .create(),
    );

    loader.finish_loading(id, &mut done_loading_event);
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, pre_loading_state: T, loading_state: T) {
    registry::create_registry::<Block>(app);

    app.add_systems((
        // Game will break without air & needs this at ID 0, so load that first
        add_air_block.in_schedule(OnEnter(pre_loading_state)),
        add_cosmos_blocks.in_schedule(OnEnter(loading_state)),
    ));
}
