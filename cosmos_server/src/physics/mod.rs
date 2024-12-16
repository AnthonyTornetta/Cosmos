//! Handles interactions between various entities + the physics worlds.
//!
//! Mostly used to move entities between worlds & sync up locations to their transforms.

use bevy::{prelude::*, utils::HashSet};
use bevy_rapier3d::{
    plugin::{RapierConfiguration, RapierContextEntityLink},
    prelude::RapierContextSimulation,
};
use cosmos_core::{
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
    physics::{
        location::{add_previous_location, handle_child_syncing, CosmosBundleSet, Location, LocationPhysicsSet, SECTOR_DIMENSIONS},
        player_world::{PlayerWorld, WorldWithin},
    },
    state::GameState,
};

mod collider_disabling;

const WORLD_SWITCH_DISTANCE: f32 = SECTOR_DIMENSIONS / 2.0;
const WORLD_SWITCH_DISTANCE_SQRD: f32 = WORLD_SWITCH_DISTANCE * WORLD_SWITCH_DISTANCE;

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

        info!("Creating new physics world!");

        let world_entity = commands
            .spawn((Name::new("Player World"), PlayerWorld { player: player_entity }, *location, link))
            .id();

        commands.entity(player_entity).insert(WorldWithin(world_entity)).insert(link);
    }
}

fn move_players_between_worlds(
    players: Query<(Entity, &Location), (With<WorldWithin>, With<Player>)>,
    mut world_within_query: Query<(&mut WorldWithin, &mut RapierContextEntityLink)>,
    mut commands: Commands,
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

                let (other_world_entity, other_body_world) =
                    world_within_query.get(other_entity).map(|(ent, world)| (ent.0, *world)).unwrap();

                let (mut world_currently_in, mut body_world) = world_within_query.get_mut(entity).unwrap();

                let distance = location.distance_sqrd(other_location);

                if distance < WORLD_SWITCH_DISTANCE_SQRD {
                    if world_currently_in.0 != other_world_entity {
                        world_currently_in.0 = other_world_entity;
                        *body_world = other_body_world;

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

                let link = create_physics_world(&mut commands);

                info!("Creating new physics world!");
                let world_entity = commands
                    .spawn((Name::new("Player World"), PlayerWorld { player: entity }, *location, link))
                    .id();

                let (mut world_within, mut body_world) = world_within_query.get_mut(entity).unwrap();

                world_within.0 = world_entity;
                *body_world = link;
            }
        }
    }
}

fn move_non_players_between_worlds(
    mut needs_world: Query<
        (Entity, &Location, Option<&mut WorldWithin>, Option<&mut RapierContextEntityLink>),
        (Without<Player>, Without<Parent>),
    >,
    players_with_worlds: Query<(&WorldWithin, &Location, &RapierContextEntityLink), With<Player>>,
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
                best_world_id = Some(*body_world);
            }
        }

        if let Some(ww) = best_ww {
            let world_link = best_world_id.expect("This should have a value if ww is some");

            if let Some(mut world_within) = maybe_within {
                let mut body_world = maybe_body_world.expect("Something should have a `RapierContextEntityLink` if it has a WorldWithin.");

                if *body_world != world_link {
                    *body_world = world_link;
                }
                if world_within.0 != ww.0 {
                    world_within.0 = ww.0;
                }
            } else {
                commands.entity(entity).insert(ww).insert(world_link);
            }
        }
    }
}

/// Removes worlds with nothing inside of them
///
/// This should be run not every frame because it can be expensive and not super necessary
fn remove_empty_worlds(
    q_rapier_entity_links: Query<&RapierContextEntityLink>,
    q_worlds: Query<(Entity, &RapierContextEntityLink), With<PlayerWorld>>,
    mut commands: Commands,
    q_rapier_contexts: Query<Entity, With<RapierContextSimulation>>,
) {
    let mut worlds = HashSet::new();

    for w in q_rapier_entity_links.iter() {
        worlds.insert(w.0);
    }

    let mut to_remove = Vec::new();
    for world_id in q_rapier_contexts.iter() {
        if !worlds.contains(&world_id) {
            to_remove.push(world_id);
        }
    }

    'world_loop: for world_id in to_remove {
        // Verify that nothing else is a part of this world before removing it.
        for body_world in q_rapier_entity_links.iter().map(|bw| bw.0) {
            if world_id == body_world {
                continue 'world_loop;
            }
        }

        for (entity, bw) in q_worlds.iter() {
            if bw.0 == world_id {
                commands.entity(entity).despawn_recursive();
            }
        }

        commands.entity(world_id).despawn_recursive();
    }
}

#[derive(Resource, Default)]
struct FixParentLocation(Vec<Entity>);

/// Bevy's query mutability rules require this query to be in a separate system than fix_location
fn find_need_fixed(
    q_added_locations_no_parent: Query<Entity, (Added<Location>, Without<PlayerWorld>, Without<Parent>)>,
    mut fix: ResMut<FixParentLocation>,
) {
    for e in q_added_locations_no_parent.iter() {
        fix.0.push(e);
    }
}

/// Handles any just-added locations that need to sync up to their transforms
fn fix_location(
    mut q_location_info: Query<(&mut Location, Option<&mut Transform>), Without<PlayerWorld>>,
    q_children: Query<&Children>,
    q_player_worlds: Query<(&Location, &WorldWithin, &RapierContextEntityLink), With<PlayerWorld>>,
    mut commands: Commands,
    q_player_world_loc: Query<&Location, With<PlayerWorld>>,
    mut fix: ResMut<FixParentLocation>,
) {
    for entity in std::mem::take(&mut fix.0) {
        // This makes the borrow checker happy
        let (location, _) = q_location_info.get(entity).expect("Guarenteed in query conditions");

        let mut best_distance = None;
        let mut best_world = None;
        let mut best_world_id = None;

        for (loc, ww, body_world) in q_player_worlds.iter() {
            let distance = location.distance_sqrd(loc);

            if best_distance.is_none() || distance < best_distance.unwrap() {
                best_distance = Some(distance);
                best_world = Some(*ww);
                best_world_id = Some(*body_world);
            }
        }

        match (best_world, best_world_id) {
            (Some(world), Some(world_id)) => {
                let Ok(loc) = q_player_world_loc.get(world.0) else {
                    warn!("A player world was missing a location");
                    continue;
                };

                recursively_fix_locations(
                    entity,
                    *loc,
                    Quat::IDENTITY,
                    &mut q_location_info,
                    &q_children,
                    &mut commands,
                    world,
                    world_id,
                );
            }
            _ => {
                warn!("Something was added with a location before a player world was created.");
                commands.entity(entity).log_components();
            }
        }
    }
}

fn recursively_fix_locations(
    entity: Entity,
    parent_loc: Location,
    mut parent_rotation: Quat,
    q_info: &mut Query<(&mut Location, Option<&mut Transform>), Without<PlayerWorld>>,
    q_children: &Query<&Children>,
    commands: &mut Commands,
    world: WorldWithin,
    world_id: RapierContextEntityLink,
) {
    let Ok((mut my_loc, my_trans)) = q_info.get_mut(entity) else {
        return;
    };

    let translation = parent_rotation.inverse().mul_vec3(-my_loc.relative_coords_to(&parent_loc));

    my_loc.last_transform_loc = Some(translation);

    commands.entity(entity).insert((world, world_id));

    if let Some(mut my_trans) = my_trans {
        my_trans.translation = translation;

        parent_rotation *= my_trans.rotation;
    } else {
        commands.entity(entity).insert((Transform::from_translation(translation),));
    }

    let Ok(children) = q_children.get(entity) else {
        return;
    };

    let my_loc = *my_loc;

    for &child in children {
        recursively_fix_locations(child, my_loc, parent_rotation, q_info, q_children, commands, world, world_id);
    }
}

/// This system syncs the locations up with their changes in transforms.
fn sync_transforms_and_locations(
    mut trans_query_no_parent: Query<(Entity, &mut Transform, &mut Location, &WorldWithin), (Without<PlayerWorld>, Without<Parent>)>,
    trans_query_with_parent: Query<(Entity, &mut Transform, &mut Location), (Without<PlayerWorld>, With<Parent>)>,
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
        if !players_query.contains(entity) && location.last_transform_loc.is_some() {
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

            let Ok(location) =
                trans_query_no_parent.get(player_entity).map(|(_, _, loc, _)| loc).or_else(|_| {
                    match trans_query_with_parent.get(player_entity) {
                        Ok((_, _, loc)) => Ok(loc),
                        Err(x) => Err(x),
                    }
                })
            else {
                // The player was just added & doesn't have a transform yet - only a location.
                continue;
            };

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

/// Fixes oddities that happen when changing parent of player
fn player_changed_parent(
    q_parent: Query<(&GlobalTransform, &Location)>,
    mut q_local_player: Query<(Entity, &mut Transform, &Location, &Parent, &PlayerWorld), (Changed<Parent>, With<Player>)>,
) {
    for (entity, mut player_trans, player_loc, parent, player_world) in q_local_player.iter_mut() {
        if entity != player_world.player {
            // This problem only effects players that control their world
            continue;
        }

        let Ok((parent_trans, parent_loc)) = q_parent.get(parent.get()) else {
            continue;
        };

        // Because the player's translation is always 0, 0, 0 we need to adjust it so the player is put into the
        // right spot in its parent.
        player_trans.translation = Quat::from_affine3(&parent_trans.affine())
            .inverse()
            .mul_vec3((*player_loc - *parent_loc).absolute_coords_f32());
    }
}

pub(super) fn register(app: &mut App) {
    collider_disabling::register(app);

    app.configure_sets(Update, LocationPhysicsSet::DoPhysics)
        .add_systems(
            Update,
            (
                player_changed_parent,
                find_need_fixed,
                fix_location,
                sync_transforms_and_locations,
                handle_child_syncing,
                add_previous_location,
                // consider for future:
                // sync_simple_transforms,
                // propagate_transforms,
            )
                .chain()
                .after(CosmosBundleSet::HandleCosmosBundles)
                .in_set(LocationPhysicsSet::DoPhysics)
                .run_if(in_state(GameState::Playing))
                .before(NetworkingSystemsSet::ReceiveMessages),
        )
        .add_systems(PostUpdate, fix_location)
        // This must be last due to commands being delayed when adding PhysicsWorlds.
        .add_systems(
            Last,
            (move_players_between_worlds, move_non_players_between_worlds, remove_empty_worlds).chain(),
        )
        .init_resource::<FixParentLocation>();
}
