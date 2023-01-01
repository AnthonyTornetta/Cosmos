use bevy::prelude::{App, Commands, Res, ResMut, SystemSet};
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
    app.add_system_set(SystemSet::on_enter(GameState::PreLoading).with_system(insert_resource))
        .add_system_set(SystemSet::on_enter(GameState::PostLoading).with_system(insert_langs));
}
