use crate::block::block_builder::BlockBuilder;
use crate::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};
use crate::registry::{self, Registry};
use bevy::ecs::schedule::StateData;
use bevy::prelude::{App, EventWriter, IntoSystemDescriptor, ResMut, SystemSet};

use super::{Block, BlockFace, BlockProperty};

pub static AIR_BLOCK_ID: u16 = 0;

pub fn add_cosmos_blocks(
    mut blocks: ResMut<Registry<Block>>,
    mut loading: ResMut<LoadingManager>,
    mut end_writer: EventWriter<DoneLoadingEvent>,
    mut start_writer: EventWriter<AddLoadingEvent>,
) {
    let id = loading.register_loader(&mut start_writer);

    blocks.register(
        BlockBuilder::new("cosmos:stone".into(), 1.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(2)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:grass".into(), 0.3)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(4)
            .set_side_uvs(BlockFace::Top, 1)
            .set_side_uvs(BlockFace::Bottom, 3)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:dirt".into(), 0.3)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(3)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:cherry_leaf".into(), 0.01)
            .add_property(BlockProperty::Transparent)
            .set_all_uvs(35)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:cherry_log".into(), 0.3)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(34)
            .set_side_uvs(BlockFace::Top, 33)
            .set_side_uvs(BlockFace::Bottom, 33)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ship_core".into(), 0.1)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::ShipOnly)
            .set_all_uvs(0)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:energy_cell".to_owned(), 0.1)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(0)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:laser_cannon".to_owned(), 0.1)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(0)
            .create(),
    );

    loading.finish_loading(id, &mut end_writer);
}

// Game will break without air & needs this at ID 0
fn add_air_block(mut blocks: ResMut<Registry<Block>>) {
    blocks.register(
        BlockBuilder::new("cosmos:air".into(), 0.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Empty)
            .create(),
    );
}

pub fn register<T: StateData + Clone + Copy>(
    app: &mut App,
    pre_loading_state: T,
    loading_state: T,
) {
    registry::register::<T, Block>(app, pre_loading_state);

    // Game will break without air & needs this at ID 0
    app.add_system_set(SystemSet::on_exit(pre_loading_state).with_system(add_air_block));

    app.add_system_set(SystemSet::on_enter(loading_state).with_system(add_cosmos_blocks));
}
