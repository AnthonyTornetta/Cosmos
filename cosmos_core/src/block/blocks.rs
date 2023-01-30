use crate::block::block_builder::BlockBuilder;
use crate::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};
use crate::registry::{self, Registry};
use bevy::ecs::schedule::StateData;
use bevy::prelude::{App, EventWriter, ResMut, SystemSet};

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
        BlockBuilder::new("cosmos:ship_core".into(), 0.3)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::ShipOnly)
            .set_all_uvs(5)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:energy_cell".to_owned(), 0.3)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(36)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:reactor".to_owned(), 0.3)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(37)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:laser_cannon".to_owned(), 0.3)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(21)
            .set_side_uvs(BlockFace::Front, 22)
            .set_side_uvs(BlockFace::Top, 23)
            .set_side_uvs(BlockFace::Bottom, 23)
            .set_side_uvs(BlockFace::Back, 24)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ship_hull".to_owned(), 0.5)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(20)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:thruster".to_owned(), 0.3)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(28)
            .set_side_uvs(BlockFace::Front, 20)
            .set_side_uvs(BlockFace::Top, 27)
            .set_side_uvs(BlockFace::Back, 26)
            .set_side_uvs(BlockFace::Bottom, 27)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:light".to_owned(), 0.1)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(16)
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

pub(crate) fn register<T: StateData + Clone + Copy>(
    app: &mut App,
    pre_loading_state: T,
    loading_state: T,
) {
    registry::register::<T, Block>(app, pre_loading_state);

    // Game will break without air & needs this at ID 0
    app.add_system_set(SystemSet::on_exit(pre_loading_state).with_system(add_air_block));

    app.add_system_set(SystemSet::on_enter(loading_state).with_system(add_cosmos_blocks));
}
