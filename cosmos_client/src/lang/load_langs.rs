use bevy::prelude::{App, Commands, OnEnter, OnExit, Res, ResMut};
use cosmos_core::{item::Item, registry::Registry};

use crate::state::game_state::GameState;

use super::Lang;

fn insert_langs(mut item_langs: ResMut<Lang<Item>>, items: Res<Registry<Item>>) {
    for item in items.iter() {
        item_langs.register(item);
    }
}

fn insert_resource(mut commands: Commands) {
    commands.insert_resource(Lang::<Item>::new("en_us", vec!["items", "blocks"]));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PreLoading), insert_resource)
        .add_systems(OnExit(GameState::PostLoading), insert_langs);
}
