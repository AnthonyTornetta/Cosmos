//! Networking logic

use bevy::prelude::*;
use cosmos_core::{
    ecs::{NeedsDespawned, despawn_needed},
    netty::sync::mapping::NetworkMapping,
};

pub mod receiver;
mod sync;

fn remove_despawned_entities(q_needs_despawned: Query<Entity, With<NeedsDespawned>>, mut mapping: ResMut<NetworkMapping>) {
    for removed in q_needs_despawned.iter() {
        mapping.remove_mapping_from_client_entity(&removed);
    }
}

pub(super) fn register(app: &mut App) {
    sync::register(app);
    receiver::register(app);

    app.add_systems(
        First,
        remove_despawned_entities
            .before(despawn_needed)
            .run_if(resource_exists::<NetworkMapping>),
    );
}
