//! Handles interactions between various entities + the physics worlds.
//!
//! Mostly used to move entities between worlds & sync up locations to their transforms.

use bevy::prelude::*;
use bevy_rapier3d::{
    plugin::{RapierConfiguration, RapierContextEntityLink},
    prelude::RapierContextSimulation,
};
use cosmos_core::{
    entities::player::Player,
    physics::{
        location::{Location, SECTOR_DIMENSIONS},
        player_world::{PlayerWorld, WorldWithin},
    },
};

mod collider_disabling;

const WORLD_SWITCH_DISTANCE: f32 = SECTOR_DIMENSIONS / 2.0;

fn create_physics_world(commands: &mut Commands) -> RapierContextEntityLink {
    let mut config = RapierConfiguration::new(1.0);
    config.gravity = Vec3::ZERO;
    let rw = commands.spawn((RapierContextSimulation::default(), config)).id();
    RapierContextEntityLink(rw)
}

/// This is used to assign a player to a specific rapier world.
pub fn assign_player_world(
    q_player_worlds: &Query<(&Location, &WorldWithin, &RapierContextEntityLink), (With<Player>, Without<Parent>)>,
    player_entity: Entity,
    location: &Location,
    commands: &mut Commands,
) {
    let mut best_distance = None;
    let mut best_world = None;
    let mut best_world_id = None;

    for (loc, ww, body_world) in q_player_worlds.iter() {
        let distance = location.distance_sqrd(loc);

        if distance <= WORLD_SWITCH_DISTANCE && (best_distance.is_none() || distance < best_distance.unwrap()) {
            best_distance = Some(distance);
            best_world = Some(*ww);
            best_world_id = Some(*body_world);
        }
    }

    if let Some(world) = best_world {
        commands
            .entity(player_entity)
            .insert(world)
            .insert(best_world_id.expect("This should never be None if world is some."));
    } else {
        let link = create_physics_world(commands);

        info!("Creating new physics world for {player_entity:?}!");

        let world_entity = commands
            .spawn((Name::new("Player World"), PlayerWorld { player: player_entity }, *location, link))
            .id();

        commands.entity(player_entity).insert(WorldWithin(world_entity)).insert(link);
    }
}

pub(super) fn register(app: &mut App) {
    collider_disabling::register(app);
}
