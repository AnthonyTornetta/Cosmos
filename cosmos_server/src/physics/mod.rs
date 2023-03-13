use bevy::{prelude::*, utils::HashSet};
use bevy_rapier3d::prelude::{BodyWorld, RapierContext, RapierWorld, DEFAULT_WORLD_ID};
use cosmos_core::{
    entities::player::Player,
    physics::{
        location::{bubble_down_locations, Location, SECTOR_DIMENSIONS},
        player_world::{PlayerWorld, WorldWithin},
    },
};

use crate::{netty::server_listener::server_listen_messages, state::GameState};

const WORLD_SWITCH_DISTANCE: f32 = SECTOR_DIMENSIONS / 2.0;
const WORLD_SWITCH_DISTANCE_SQRD: f32 = WORLD_SWITCH_DISTANCE * WORLD_SWITCH_DISTANCE;

pub fn assign_player_world(
    player_worlds: &Query<(&Location, &WorldWithin, &BodyWorld), (With<Player>, Without<Parent>)>,
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
            .insert(BodyWorld {
                world_id: best_world_id.expect("This should never be None if world is some."),
            });
    } else {
        let world_id = rapier_context.add_world(RapierWorld::default());

        println!("Added world!!!");

        let world_entity = commands
            .spawn((
                PlayerWorld {
                    player: player_entity,
                },
                *location,
                BodyWorld { world_id },
            ))
            .id();

        commands
            .entity(player_entity)
            .insert(WorldWithin(world_entity))
            .insert(BodyWorld { world_id });
    }
}

pub fn move_players_between_worlds(
    players: Query<(Entity, &Location), (With<WorldWithin>, With<Player>)>,
    mut world_within_query: Query<(&mut WorldWithin, &mut BodyWorld)>,

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

                // println!("Distance: {distance} vs {WORLD_SWITCH_DISTANCE_SQRD}");

                if distance < WORLD_SWITCH_DISTANCE_SQRD {
                    if world_currently_in.0 != other_world_entity {
                        world_currently_in.0 = other_world_entity;
                        body_world.world_id = other_body_world;

                        println!("Swapped to other player's world!");

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
                        BodyWorld { world_id },
                    ))
                    .id();

                let (mut world_within, mut body_world) =
                    world_within_query.get_mut(entity).unwrap();

                world_within.0 = world_entity;
                body_world.world_id = world_id;

                println!("BOOM! NEW WORLD CREATED -- world id: {world_id}!");
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
            Option<&mut BodyWorld>,
        ),
        (Without<Player>, Without<Parent>),
    >,
    players_with_worlds: Query<(&WorldWithin, &Location, &BodyWorld), With<Player>>,
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
                    .expect("Something should have a BodyWorld if it has a WorldWithin.");

                if body_world.world_id != world_id {
                    println!("CHANGING WORLD!");
                    body_world.world_id = world_id;
                }
                if world_within.0 != ww.0 {
                    world_within.0 = ww.0;
                }
            } else {
                commands
                    .entity(entity)
                    .insert(ww)
                    .insert(BodyWorld { world_id });
            }
        }
    }
}

fn remove_empty_worlds(
    query: Query<&BodyWorld>,
    worlds_query: Query<(Entity, &BodyWorld), With<PlayerWorld>>,
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

    for world_id in to_remove {
        for (entity, bw) in worlds_query.iter() {
            if bw.world_id == world_id {
                commands.entity(entity).despawn_recursive();
            }
        }

        println!("Removed world {world_id}");

        context
            .remove_world(world_id)
            .expect("This world was just found to exist, so it should.");
    }
}

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
            location.apply_updates(transform.translation);
        }
    }
    for (entity, transform, mut location) in trans_query_with_parent.iter_mut() {
        // Server transforms for players should NOT be applied to the location.
        // The location the client sent should override it.
        if !players_query.contains(entity) {
            location.apply_updates(transform.translation);
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
                .map(|x| x.2)
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
                    location.last_transform_loc = transform.translation;
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

pub(crate) fn register(app: &mut App) {
    app.add_systems(
        (
            // remove_empty_worlds,
            // If it's not after server_listen_messages, some noticable jitter can happen
            sync_transforms_and_locations.after(server_listen_messages),
            bubble_down_locations.after(sync_transforms_and_locations),
            move_players_between_worlds.after(bubble_down_locations),
            move_non_players_between_worlds.after(move_players_between_worlds),
        )
            .in_set(OnUpdate(GameState::Playing)),
    );
}
