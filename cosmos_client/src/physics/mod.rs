//! Handles client-only physics stuff

use bevy::{
    app::Startup,
    math::Vec3,
    prelude::{App, Commands, Entity, Query, Update, With, Without},
};
use bevy_rapier3d::{
    plugin::{RapierConfiguration, RapierContextEntityLink},
    prelude::RapierContextSimulation,
};
use cosmos_core::physics::{location::Location, player_world::PlayerWorld};

mod collider_disabling;

/// Everything in the client is in the same world
fn add_world_within(
    query: Query<Entity, (With<Location>, Without<RapierContextEntityLink>)>,
    mut commands: Commands,
    player_world: Query<Entity, With<PlayerWorld>>,
) {
    if let Ok(pw) = player_world.single() {
        for entity in query.iter() {
            commands.entity(entity).insert(RapierContextEntityLink(pw));
        }
    }
}

fn remove_gravity(mut commands: Commands, rapier_access: Query<Entity, With<RapierContextSimulation>>) {
    for e in rapier_access.iter() {
        let mut config = RapierConfiguration::new(1.0);
        config.gravity = Vec3::ZERO;

        commands.entity(e).insert(config);
    }
}

pub(super) fn register(app: &mut App) {
    collider_disabling::register(app);

    app.add_systems(Update, add_world_within).add_systems(Startup, remove_gravity);
}
