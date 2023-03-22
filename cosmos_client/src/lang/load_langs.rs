use bevy::prelude::{App, Commands, IntoSystemAppConfig, OnEnter, OnExit, Res, ResMut};
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

pub fn register(app: &mut App) {
    app.add_system(insert_resource.in_schedule(OnEnter(GameState::PreLoading)))
        .add_system(insert_langs.in_schedule(OnExit(GameState::PostLoading)));
}
