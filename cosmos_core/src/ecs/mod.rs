//! Contains utilities that make interacting with the bevy ECS easier & less
//! prone to problems.

use bevy::prelude::{
    App, Commands, Component, CoreSet, DespawnRecursiveExt, Entity, IntoSystemConfig, Query, With,
};

#[derive(Component, Debug)]
/// Marks an entity that needs to be recurisvely despawned.
///
/// The entity will be despawned in the PostUpdate base set to avoid crashes.
pub struct NeedsDespawned;

/// Recursively despawns all entities that need despawned in the PostUpdate set.
pub fn despawn_needed(
    mut commands: Commands,
    needs_despawned_query: Query<Entity, With<NeedsDespawned>>,
) {
    for ent in needs_despawned_query.iter() {
        commands.entity(ent).despawn_recursive();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(despawn_needed.in_base_set(CoreSet::PostUpdate));
}
