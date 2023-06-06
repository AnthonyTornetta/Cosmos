//! Handles client-only physics stuff

use bevy::prelude::{App, Commands, Entity, Query, With, Without};
use cosmos_core::physics::{
    location::Location,
    player_world::{PlayerWorld, WorldWithin},
};

/// Everything in the client is in the same world
fn add_world_within(
    query: Query<Entity, (With<Location>, Without<WorldWithin>)>,
    mut commands: Commands,
    player_world: Query<Entity, With<PlayerWorld>>,
) {
    if let Ok(pw) = player_world.get_single() {
        for entity in query.iter() {
            commands.entity(entity).insert(WorldWithin(pw));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(add_world_within);
}
