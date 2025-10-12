//! Contains utilities that make interacting with the bevy ECS easier & less
//! prone to problems.

pub mod commands;
pub mod data;
pub mod mut_events;
pub mod sets;

use bevy::{ecs::query::QueryFilter, prelude::*};

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
        commands.entity(ent).despawn();
    }
}

/// Runs `commands.init_resource` in a system. Useful for adding `run_if` statements quickly
pub fn init_resource<R: Resource + Default>(mut commands: Commands) {
    commands.init_resource::<R>();
}

/// A system that removes the given resource
pub fn remove_resource<R: Resource>(mut commands: Commands) {
    commands.remove_resource::<R>();
}

/// A system that adds a resource when entring this state, and removes it when exiting this state.
pub fn add_statebound_resource<R: Resource + Default, S: States + Clone + Copy>(app: &mut App, state: S) {
    add_multi_statebound_resource::<R, S>(app, state, state)
}

/// A system that adds a resource when entring the `add_state` state, and removes it when exiting the `remove_on_exit_state` state.
pub fn add_multi_statebound_resource<R: Resource + Default, S: States>(app: &mut App, add_state: S, remove_on_exit_state: S) {
    app.add_systems(OnEnter(add_state), init_resource::<R>)
        .add_systems(OnExit(remove_on_exit_state), remove_resource::<R>);
}

/// Returns the live-computed [`GlobalTransform`] for this entity.
///
/// Returns [`None`] if the entity is in an invalid transform heirarchy, so you can typically
/// `expect` this result unless it could be in an invalid heirarchy.
///
/// This is used when the [`GlobalTransform`] is too inaccurate for usage.
pub fn compute_totally_accurate_global_transform<F: QueryFilter>(
    entity: Entity,
    q_trans: &Query<(&Transform, Option<&ChildOf>), F>,
) -> Option<GlobalTransform> {
    let (ct, mut maybe_parent) = q_trans.get(entity).ok()?;
    let mut g_trans = GlobalTransform::default() * *ct;
    while let Some(parent) = maybe_parent {
        let (parent_trans, new_maybe_parent) = q_trans.get(parent.parent()).ok()?;

        g_trans = *parent_trans * g_trans;
        maybe_parent = new_maybe_parent;
    }
    Some(g_trans)
}

/// A utility system to automatically [`Name`] this component to the given name when added.
///
/// Usage:
/// ```rs
/// app.add_systems(Update, name::<LaserCannonSystem>("Laser Cannon System"));
/// ```
pub fn name<T: Component>(name: &'static str) -> impl Fn(Commands, Query<Entity, Added<T>>) {
    move |mut commands: Commands, q: Query<Entity, Added<T>>| {
        for e in q.iter() {
            commands.entity(e).insert(Name::new(name));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(First, despawn_needed);
    sets::register(app);
    data::register(app);
}
