//! Bevy ECS utilities

use bevy::app::PostUpdate;
use bevy::prelude::App;
use bevy::prelude::Commands;
use bevy::prelude::Component;
use bevy::prelude::Entity;
use bevy::prelude::Query;
use bevy::prelude::With;
use bevy::prelude::Without;
use bevy::reflect::Reflect;

use crate::ecs::NeedsDespawned;

/// When the entity referenced doesn't exist, then the entity this is attached to will be flagged
/// for deletion
#[derive(Component, Reflect, Debug)]
pub struct DespawnWith(pub Entity);

fn despawn_with_handler(
    mut commands: Commands,
    q_entity: Query<Entity, With<NeedsDespawned>>,
    q_despawn_with: Query<(Entity, &DespawnWith), Without<NeedsDespawned>>,
) {
    for (ent, despawn_with) in q_despawn_with.iter() {
        if q_entity.contains(despawn_with.0) {
            commands.entity(ent).insert(NeedsDespawned);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PostUpdate, despawn_with_handler);

    app.register_type::<DespawnWith>();
}
