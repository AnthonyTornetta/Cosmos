//! Contains utilities that make interacting with the bevy ECS easier & less
//! prone to problems.

pub mod bundles;
pub mod mut_events;

use bevy::prelude::{App, Commands, Component, DespawnRecursiveExt, Entity, First, Query, With};

#[derive(Component, Debug)]
/// Marks an entity that needs to be recurisvely despawned.
///
/// This does NOT make the entity get saved. Add `NeedsSaved` in addition to this component
/// to save & despawn the entity. If you just want to save the entity, but not despawn it, you can just
/// add `NeedsSaved`.
///
/// ## NOTE:
/// If an entity is marked with `NeedsDespawned` and was previously saved, the save file will be deleted.
/// To prevent this, mark it with `NeedsSaved`.
///
/// The entity will be despawned in `CoreSet::First` base set to avoid crashes.
pub struct NeedsDespawned;

/// Recursively despawns all entities that need despawned in `CoreSet::First`.
pub fn despawn_needed(mut commands: Commands, needs_despawned_query: Query<Entity, With<NeedsDespawned>>) {
    for ent in needs_despawned_query.iter() {
        commands.entity(ent).despawn_recursive();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(First, despawn_needed);
}
