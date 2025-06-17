use bevy::prelude::*;
use cosmos_core::{
    crafting::recipes::basic_fabricator::SyncBasicFabricatorRecipesEvent,
    ecs::sets::FixedUpdateSet,
    netty::sync::events::client_event::NettyEventReceived,
};

fn sync_recipes(mut commands: Commands, mut nevr: EventReader<NettyEventReceived<SyncBasicFabricatorRecipesEvent>>) {
    for ev in nevr.read() {
        let recipes = ev.0.clone();
        commands.insert_resource(recipes);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(FixedUpdate, sync_recipes.in_set(FixedUpdateSet::Main));
}
