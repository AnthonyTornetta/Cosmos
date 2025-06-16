use bevy::prelude::*;
use cosmos_core::{
    crafting::recipes::basic_fabricator::SyncBasicFabricatorRecipesEvent,
    netty::{sync::events::client_event::NettyEventReceived, system_sets::NetworkingSystemsSet},
};

fn sync_recipes(mut commands: Commands, mut nevr: EventReader<NettyEventReceived<SyncBasicFabricatorRecipesEvent>>) {
    for ev in nevr.read() {
        let recipes = ev.0.clone();
        commands.insert_resource(recipes);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, sync_recipes.in_set(NetworkingSystemsSet::Between));
}
