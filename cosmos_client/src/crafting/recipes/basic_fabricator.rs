use bevy::prelude::*;
use cosmos_core::{
    crafting::recipes::basic_fabricator::SyncBasicFabricatorRecipesMessage, ecs::sets::FixedUpdateSet,
    netty::sync::events::client_event::NettyMessageReceived,
};

fn sync_recipes(mut commands: Commands, mut nevr: MessageReader<NettyMessageReceived<SyncBasicFabricatorRecipesMessage>>) {
    for ev in nevr.read() {
        let recipes = ev.0.clone();
        commands.insert_resource(recipes);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(FixedUpdate, sync_recipes.in_set(FixedUpdateSet::Main));
}
