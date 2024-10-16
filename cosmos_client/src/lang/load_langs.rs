use bevy::prelude::{App, Commands, OnEnter, OnExit, Res, ResMut};
use cosmos_core::{block::Block, item::Item, registry::Registry, state::GameState};

use super::Lang;

fn insert_langs(
    mut item_langs: ResMut<Lang<Item>>,
    mut block_langs: ResMut<Lang<Block>>,
    blocks: Res<Registry<Block>>,
    items: Res<Registry<Item>>,
) {
    for item in items.iter() {
        item_langs.register(item);
    }

    for block in blocks.iter() {
        block_langs.register(block);
    }
}

fn insert_resource(mut commands: Commands) {
    commands.insert_resource(Lang::<Item>::new("en_us", vec!["items", "blocks"]));
    commands.insert_resource(Lang::<Block>::new("en_us", vec!["blocks"]));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PreLoading), insert_resource)
        .add_systems(OnExit(GameState::PostLoading), insert_langs);
}
