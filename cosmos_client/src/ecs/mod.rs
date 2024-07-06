//! Some utility systems that make working with the bevy ECS a bit easier in a multiplayer environment.

use bevy::prelude::{
    resource_exists, App, Children, Commands, Entity, First, IntoSystemConfigs, OnEnter, OnExit, Query, ResMut, Resource, With,
};
use cosmos_core::{
    ecs::{despawn_needed, NeedsDespawned},
    netty::sync::mapping::NetworkMapping,
};

use crate::state::game_state::GameState;

/// Recursively removes the networking mappings to all entities that are about to be despawned
pub fn remove_mappings(
    needs_despawned_query: Query<Entity, With<NeedsDespawned>>,
    children_query: Query<&Children>,
    mut network_mapping: ResMut<NetworkMapping>,
) {
    for ent in needs_despawned_query.iter() {
        recursively_remove(ent, &children_query, &mut network_mapping);
    }
}

fn recursively_remove(entity: Entity, children_query: &Query<&Children>, network_mapping: &mut NetworkMapping) {
    if let Ok(children) = children_query.get(entity) {
        children
            .iter()
            .copied()
            .for_each(|c| recursively_remove(c, children_query, network_mapping));
    }

    network_mapping.remove_mapping_from_client_entity(&entity);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        First,
        remove_mappings.after(despawn_needed).run_if(resource_exists::<NetworkMapping>),
    );
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
pub fn add_statebound_resource<R: Resource + Default>(app: &mut App, state: GameState) {
    add_multi_statebound_resource::<R>(app, state, state)
}

/// A system that adds a resource when entring the `add_state` state, and removes it when exiting the `remove_on_exit_state` state.
pub fn add_multi_statebound_resource<R: Resource + Default>(app: &mut App, add_state: GameState, remove_on_exit_state: GameState) {
    app.add_systems(OnEnter(add_state), init_resource::<R>)
        .add_systems(OnExit(remove_on_exit_state), remove_resource::<R>);
}
