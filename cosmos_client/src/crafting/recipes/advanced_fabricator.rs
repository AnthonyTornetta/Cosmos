use bevy::prelude::*;
use cosmos_core::{
    crafting::recipes::advanced_fabricator::SyncAdvancedFabricatorRecipesEvent, ecs::sets::FixedUpdateSet,
    netty::sync::events::client_event::NettyMessageReceived,
};

fn sync_recipes(mut commands: Commands, mut nevr: EventReader<NettyMessageReceived<SyncAdvancedFabricatorRecipesEvent>>) {
    for ev in nevr.read() {
        let recipes = ev.0.clone();
        commands.insert_resource(recipes);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(FixedUpdate, sync_recipes.in_set(FixedUpdateSet::Main));
}
