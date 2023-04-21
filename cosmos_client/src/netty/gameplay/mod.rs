use bevy::prelude::{App, Entity, Query, ResMut, Without};
use cosmos_core::netty::NoSendEntity;

use super::mapping::NetworkMapping;

mod receiver;
mod sync;

fn remove_despawned_entities(
    entities: Query<Entity, Without<NoSendEntity>>,
    mapping: Option<ResMut<NetworkMapping>>,
) {
    if let Some(mut mapping) = mapping {
        mapping.only_keep_these_entities(&entities.iter().collect::<Vec<Entity>>());
    }
}

pub(super) fn register(app: &mut App) {
    sync::register(app);
    receiver::register(app);

    app.add_system(remove_despawned_entities);
}
