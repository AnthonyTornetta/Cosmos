use bevy::{
    app::Update,
    prelude::{App, Commands, EventReader, IntoSystemConfigs},
};
use cosmos_core::{
    crafting::recipes::advanced_weapons_fabricator::SyncAdvancedWeaponsFabricatorRecipesEvent,
    netty::{sync::events::client_event::NettyEventReceived, system_sets::NetworkingSystemsSet},
};

fn sync_recipes(mut commands: Commands, mut nevr: EventReader<NettyEventReceived<SyncAdvancedWeaponsFabricatorRecipesEvent>>) {
    for ev in nevr.read() {
        let recipes = ev.0.clone();
        commands.insert_resource(recipes);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, sync_recipes.in_set(NetworkingSystemsSet::Between));
}
