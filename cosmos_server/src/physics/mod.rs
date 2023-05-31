//! Handles interactions between various entities + the physics worlds.
//!
//! Mostly used to move entities between worlds & sync up locations to their transforms.

use bevy::{prelude::*, utils::HashSet};
use bevy_rapier3d::prelude::{PhysicsWorld, RapierContext, RapierWorld, DEFAULT_WORLD_ID};
use cosmos_core::{
    entities::player::Player,
    physics::{
        location::{handle_child_syncing, Location, SECTOR_DIMENSIONS},
        player_world::{PlayerWorld, WorldWithin},
    },
};

use crate::state::GameState;

const WORLD_SWITCH_DISTANCE: f32 = SECTOR_DIMENSIONS / 2.0;
const WORLD_SWITCH_DISTANCE_SQRD: f32 = WORLD_SWITCH_DISTANCE * WORLD_SWITCH_DISTANCE;

/// This is used to assign a player to a specific rapier world.
pub fn assign_player_world(
    player_worlds: &Query<
        (&Location, &WorldWithin, &PhysicsWorld),
        (With<Player>, Without<Parent>),
    >,
    player_entity: Entity,
    location: &Location,
    commands: &mut Commands,
    rapier_context: &mut RapierContext,
) {
    let mut best_distance = None;
    let mut best_world = None;
    let mut best_world_id = None;

    for (loc, ww, body_world) in player_worlds.iter() {
        let distance = location.distance_sqrd(loc);

        if distance <= WORLD_SWITCH_DISTANCE
            && (best_distance.is_none() || distance < best_distance.unwrap())
        {
            best_distance = Some(distance);
            best_world = Some(*ww);
            best_world_id = Some(body_world.world_id);
        }
    }

    if let Some(world) = best_world {
        commands
            .entity(player_entity)
            .insert(world)
            .insert(PhysicsWorld {
                world_id: best_world_id.expect("This should never be None if world is some."),
            });
    } else {
        let world_id = rapier_context.add_world(RapierWorld::default());

        let world_entity = commands
            .spawn((
                PlayerWorld {
                    player: player_entity,
                },
                *location,
                PhysicsWorld { world_id },
            ))
            .id();

        commands
            .entity(player_entity)
            .insert(WorldWithin(world_entity))
            .insert(PhysicsWorld { world_id });
    }
}

fn move_players_between_worlds(
    players: Query<(Entity, &Location), (With<WorldWithin>, With<Player>)>,
    mut world_within_query: Query<(&mut WorldWithin, &mut PhysicsWorld)>,

    mut commands: Commands,
    mut rapier_context: ResMut<RapierContext>,
) {
    let mut changed = true;

    let mut getting_new_world = Vec::new();

    while changed {
        changed = false;

        for (entity, location) in players.iter() {
            let mut needs_new_world = false;

            for (other_entity, other_location) in players.iter() {
                if other_entity == entity || getting_new_world.contains(&other_entity) {
                    continue;
                }

                let (other_world_entity, other_body_world) = world_within_query
                    .get(other_entity)
                    .map(|(ent, world)| (ent.0, world.world_id))
                    .unwrap();

                let (mut world_currently_in, mut body_world) =
                    world_within_query.get_mut(entity).unwrap();

                let distance = location.distance_sqrd(other_location);

                if distance < WORLD_SWITCH_DISTANCE_SQRD {
                    if world_currently_in.0 != other_world_entity {
                        world_currently_in.0 = other_world_entity;
                        body_world.world_id = other_body_world;

                        needs_new_world = false;
                        changed = true;
                    }
                    break;
                } else if world_currently_in.0 == other_world_entity {
                    needs_new_world = true;
                }
            }

            if needs_new_world {
                getting_new_world.push(entity);

                let world_id = rapier_context.add_world(RapierWorld::default());

                let world_entity = commands
                    .spawn((
                        PlayerWorld { player: entity },
                        *location,
                        PhysicsWorld { world_id },
                    ))
                    .id();

                let (mut world_within, mut body_world) =
                    world_within_query.get_mut(entity).unwrap();

                world_within.0 = world_entity;
                body_world.world_id = world_id;
            }
        }
    }
}

fn move_non_players_between_worlds(
    mut needs_world: Query<
        (
            Entity,
            &Location,
            Option<&mut WorldWithin>,
            Option<&mut PhysicsWorld>,
        ),
        (Without<Player>, Without<Parent>),
    >,
    players_with_worlds: Query<(&WorldWithin, &Location, &PhysicsWorld), With<Player>>,
    mut commands: Commands,
) {
    for (entity, location, maybe_within, maybe_body_world) in needs_world.iter_mut() {
        let mut best_ww = None;
        let mut best_dist = None;
        let mut best_world_id = None;

        for (ww, player_loc, body_world) in players_with_worlds.iter() {
            let dist = player_loc.distance_sqrd(location);

            if best_ww.is_none() || dist < best_dist.unwrap() {
                best_ww = Some(*ww);
                best_dist = Some(dist);
                best_world_id = Some(body_world.world_id);
            }
        }

        if let Some(ww) = best_ww {
            let world_id = best_world_id.expect("This should have a value if ww is some");

            if let Some(mut world_within) = maybe_within {
                let mut body_world = maybe_body_world
                    .expect("Something should have a PhysicsWorld if it has a WorldWithin.");

                if body_world.world_id != world_id {
                    body_world.world_id = world_id;
                }
                if world_within.0 != ww.0 {
                    world_within.0 = ww.0;
                }
            } else {
                commands
                    .entity(entity)
                    .insert(ww)
                    .insert(PhysicsWorld { world_id });
            }
        }
    }
}

/// Removes worlds with nothing inside of them
///
/// This should be run not every frame because it can be expensive and not super necessary
fn remove_empty_worlds(
    query: Query<&PhysicsWorld>,
    worlds_query: Query<(Entity, &PhysicsWorld), With<PlayerWorld>>,
    everything_query: Query<&PhysicsWorld>,
    mut context: ResMut<RapierContext>,
    mut commands: Commands,
) {
    let mut worlds = HashSet::new();

    for w in query.iter() {
        worlds.insert(w.world_id);
    }

    let mut to_remove = Vec::new();
    for (world_id, _) in context.worlds.iter() {
        if *world_id != DEFAULT_WORLD_ID && !worlds.contains(world_id) {
            to_remove.push(*world_id);
        }
    }

    'world_loop: for world_id in to_remove {
        // Verify that nothing else is a part of this world before removing it.
        for body_world in everything_query.iter().map(|bw| bw.world_id) {
            if world_id == body_world {
                continue 'world_loop;
            }
        }

        for (entity, bw) in worlds_query.iter() {
            if bw.world_id == world_id {
                commands.entity(entity).despawn_recursive();
            }
        }

        context
            .remove_world(world_id)
            .expect("This world was just found to exist, so it should.");
    }
}

/// Handles any just-added locations that need to sync up to their transforms
fn fix_location(
    mut query: Query<(Entity, &mut Location), (Added<Location>, Without<PlayerWorld>)>,
    player_worlds: Query<(&Location, &WorldWithin, &PhysicsWorld), With<PlayerWorld>>,
    mut commands: Commands,
    player_world_loc_query: Query<&Location, With<PlayerWorld>>,
) {
    for (entity, mut location) in query.iter_mut() {
        let mut best_distance = None;
        let mut best_world = None;
        let mut best_world_id = None;

        for (loc, ww, body_world) in player_worlds.iter() {
            let distance = location.distance_sqrd(loc);

            if best_distance.is_none() || distance < best_distance.unwrap() {
                best_distance = Some(distance);
                best_world = Some(*ww);
                best_world_id = Some(body_world.world_id);
            }
        }

        match (best_world, best_world_id) {
            (Some(world), Some(world_id)) => {
                if let Ok(loc) = player_world_loc_query.get(world.0) {
                    let transform = Transform::from_translation(location.relative_coords_to(loc));

                    location.last_transform_loc = Some(transform.translation);

                    commands.entity(entity).insert((
                        TransformBundle::from_transform(transform),
                        world,
                        PhysicsWorld { world_id },
                    ));
                } else {
                    warn!("A player world was missing a location");
                }
            }
            _ => {
                warn!("Something was added with a location before a player world was registered.")
            }
        }
    }
}

/// This system syncs the locations up with their changes in transforms.
fn sync_transforms_and_locations(
    mut trans_query_no_parent: Query<
        (Entity, &mut Transform, &mut Location, &WorldWithin),
        (Without<PlayerWorld>, Without<Parent>),
    >,
    mut trans_query_with_parent: Query<
        (Entity, &mut Transform, &mut Location),
        (Without<PlayerWorld>, With<Parent>),
    >,
    players_query: Query<(&WorldWithin, Entity), With<Player>>,
    everything_query: Query<(&WorldWithin, Entity)>,
    parent_query: Query<&Parent>,
    entity_query: Query<Entity>,
    mut world_query: Query<(Entity, &mut PlayerWorld, &mut Location)>,

    mut commands: Commands,
) {
    for (entity, transform, mut location, _) in trans_query_no_parent.iter_mut() {
        // Server transforms for players should NOT be applied to the location.
        // The location the client sent should override it.
        if !players_query.contains(entity) {
            if location.last_transform_loc.is_some() {
                location.apply_updates(transform.translation);
            }
        }
    }
    for (entity, transform, mut location) in trans_query_with_parent.iter_mut() {
        // Server transforms for players should NOT be applied to the location.
        // The location the client sent should override it.
        if !players_query.contains(entity) {
            if location.last_transform_loc.is_some() {
                location.apply_updates(transform.translation);
            }
        }
    }

    for (world_entity, mut world, mut world_location) in world_query.iter_mut() {
        if let Ok(mut player_entity) = entity_query.get(world.player) {
            while let Ok(parent) = parent_query.get(player_entity) {
                let parent_entity = parent.get();
                if trans_query_no_parent.contains(parent_entity) {
                    player_entity = parent.get();
                } else {
                    break;
                }
            }

            let location = trans_query_no_parent
                .get(player_entity)
                .map(|(_, _, loc, _)| loc)
                .or_else(|_| match trans_query_with_parent.get(player_entity) {
                    Ok((_, _, loc)) => Ok(loc),
                    Err(x) => Err(x),
                })
                .expect("The above loop guarantees this is valid");

            world_location.set_from(location);

            // Update transforms of objects within this world.
            for (_, mut transform, mut location, world_within) in trans_query_no_parent.iter_mut() {
                if world_within.0 == world_entity {
                    transform.translation = world_location.relative_coords_to(&location);
                    location.last_transform_loc = Some(transform.translation);
                }
            }
        } else {
            // The player has disconnected
            // Either: Find a new player to have the world
            // Or: Move everything over to the closest world by removing the WorldWithin component

            let mut found = false;
            // find player
            for (world_within, entity) in players_query.iter() {
                if world_within.0 == world_entity {
                    world.player = entity;
                    found = true;
                    break;
                }
            }

            // No suitable player found, find another world to move everything to
            if !found {
                for (world_within, entity) in everything_query.iter() {
                    if world_within.0 == world_entity {
                        commands.entity(entity).remove::<WorldWithin>();
                    }
                }
                commands.entity(world_entity).despawn_recursive();
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        (
            // If it's not after server_listen_messages, some noticable jitter can happen
            fix_location, //.after(server_listen_messages),
            move_players_between_worlds,
            move_non_players_between_worlds,
        )
            .chain()
            .in_base_set(CoreSet::Last),
    )
    .add_systems(
        (sync_transforms_and_locations, handle_child_syncing)
            .chain()
            .in_set(OnUpdate(GameState::Playing)),
    )
    // This must be last due to commands being delayed when adding PhysicsWorlds.
    .add_system(remove_empty_worlds.in_base_set(CoreSet::Last));
}
