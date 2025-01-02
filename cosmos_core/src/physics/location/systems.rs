//!
//!
//! - Apply set position
//! - Set location based on transform
//!     This can only be done for children - doesn't make sense for parents
//! - Update Worlds
//! - sync transforms and locations
//!
//!

use bevy::{prelude::*, transform::systems::propagate_transforms, utils::HashSet};
use bevy_rapier3d::{
    plugin::{DefaultRapierContext, RapierConfiguration, RapierContextEntityLink},
    prelude::RapierContextSimulation,
};

use crate::{
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
    physics::{
        location::LastTransformTranslation,
        player_world::{PlayerWorld, WorldWithin},
    },
    state::GameState,
};

use super::{CosmosBundleSet, Location, LocationPhysicsSet, Sector, SetPosition, SECTOR_DIMENSIONS};

#[derive(Component)]
struct Anchor;

fn calc_global_trans(entity: Entity, q_trans: &Query<(&Transform, Option<&Parent>)>) -> Option<Transform> {
    let Ok((trans, parent)) = q_trans.get(entity) else {
        error!("Inconsistent transform heirarchy!");
        return None;
    };

    if let Some(parent) = parent {
        calc_global_trans(parent.get(), q_trans).map(|t| t * *trans)
    } else {
        Some(*trans)
    }
}

fn loc_from_trans(
    entity: Entity,
    q_trans: &Query<(&Transform, Option<&Parent>)>,
    q_x: &Query<(Entity, Option<&Location>, Option<&SetPosition>)>,
) -> Option<Location> {
    let (entity, loc, set_pos) = q_x.get(entity).expect("Invalid entity given");

    match set_pos {
        None | Some(SetPosition::Location) => loc.copied(),
        Some(SetPosition::Transform) => calc_global_trans(entity, &q_trans).map(|t| Location::new(t.translation, Sector::ZERO)),
    }
}

#[derive(Component)]
struct SetTransformBasedOnLocationFlag;

fn apply_set_position(
    q_location_added: Query<(Entity, Option<&Parent>), (Without<SetPosition>, Added<Location>)>,
    q_set_position: Query<(Entity, &SetPosition)>,
    q_x: Query<(Entity, Option<&Location>, Option<&SetPosition>)>,
    q_trans: Query<(&Transform, Option<&Parent>)>,
    q_g_trans: Query<&GlobalTransform>,
    mut commands: Commands,
) {
    const TOP_LEVEL_SET_POS: SetPosition = SetPosition::Location;
    const CHILD_SET_POS: SetPosition = SetPosition::Transform;

    for (entity, set_pos) in q_location_added
        .iter()
        .map(|(ent, parent)| (ent, if parent.is_some() { &CHILD_SET_POS } else { &TOP_LEVEL_SET_POS }))
        .chain(q_set_position.iter())
    {
        match set_pos {
            SetPosition::Location => {
                let mut ecmds = commands.entity(entity);
                if let Ok(g_trans) = q_g_trans.get(entity) {
                    ecmds.insert(LastTransformTranslation(g_trans.translation()));
                }
                ecmds.insert(SetTransformBasedOnLocationFlag).remove::<SetPosition>();
            }
            SetPosition::Transform => {
                if let Some(loc_from_trans) = loc_from_trans(entity, &q_trans, &q_x) {
                    if let Ok(g_trans) = q_g_trans.get(entity) {
                        commands
                            .entity(entity)
                            .insert((loc_from_trans, LastTransformTranslation(g_trans.translation())))
                            .remove::<SetPosition>();
                    }
                }
            }
        }
    }
}

// fn calculate_global_translation(entity: Entity, q_trans: &Query<(&Transform, Option<&Parent>)>) -> Transform {
//     let Ok((trans, parent)) = q_trans.get(entity) else {
//         error!("Bad heirarchy - parent missing Transform");
//         return Vec3::ZERO;
//     };
//
//     if let Some(parent) = parent {
//         calc_global_trans(parent.get(), q_trans)
//     }
// }

// fn set_transform_based_on_location(
//     mut q_q: Query<
//         (
//             &mut Transform,
//             &GlobalTransform,
//             &mut Location,
//             &WorldWithin,
//             &mut LastTransformTranslation,
//             Option<&Parent>,
//         ),
//         Without<PlayerWorld>,
//     >,
//     q_g_trans: Query<&GlobalTransform>,
//     q_physics_world: Query<&Location, With<PlayerWorld>>,
// ) {
//     for (mut trans, g_trans, mut loc, world_within, mut last_translation, parent) in q_q.iter_mut() {
//         let Ok(pw_loc) = q_physics_world.get(world_within.0) else {
//             continue;
//         };
//
//         let parent_g_trans = if let Some(parent) = parent {
//             q_g_trans.get(parent.get()).map(|x| *x).unwrap_or_default()
//         } else {
//             Default::default()
//         };
//
//         let absolute_coords_relative_to_world = (*loc - *pw_loc).absolute_coords_f32();
//
//         let local_coords = parent_g_trans.rotation().inverse() * (absolute_coords_relative_to_world - parent_g_trans.translation());
//         trans.translation = local_coords;
//
//         let delta = g_trans.translation() - last_translation.0;
//         *loc = *loc + delta;
//         last_translation.0 = g_trans.translation();
//     }
// }

fn do_the_kids(
    parent_g_trans: Vec3,
    parent_g_rot: Quat,
    mut q_trans: Query<
        (
            Entity,
            &mut Transform,
            &Location,
            &mut LastTransformTranslation,
            &WorldWithin,
            Option<&Children>,
        ),
        (Without<PlayerWorld>, With<Parent>),
    >,
) {
}

fn ensure_worlds_have_anchors(
    mut trans_query_no_parent: Query<
        (
            Entity,
            &mut Transform,
            &Location,
            &mut LastTransformTranslation,
            &WorldWithin,
            // Option<&Children>,
        ),
        (Without<PlayerWorld>, Without<Parent>),
    >,
    trans_query_with_parent: Query<(Entity, &mut Transform, &mut Location), (Without<PlayerWorld>, With<Parent>)>,
    players_query: Query<(&WorldWithin, Entity), With<Anchor>>,
    everything_query: Query<(&WorldWithin, Entity)>,
    parent_query: Query<&Parent>,
    entity_query: Query<Entity>,
    mut world_query: Query<(Entity, &mut PlayerWorld, &mut Location)>,

    mut commands: Commands,
) {
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

            let Ok(location) = trans_query_no_parent.get(player_entity).map(|(_, _, loc, _, _)| loc).or_else(|_| {
                match trans_query_with_parent.get(player_entity) {
                    Ok((_, _, loc)) => Ok(loc),
                    Err(x) => Err(x),
                }
            }) else {
                // The player was just added & doesn't have a transform yet - only a location.
                continue;
            };

            *world_location = *location;

            // // Update transforms of objects within this world.
            // for (_, mut transform, location, mut last_transform_loc, world_within) in trans_query_no_parent.iter_mut() {
            //     if world_within.0 == world_entity {
            //         info!("WORLDO PRE: {:?}", transform.translation);
            //         transform.translation = world_location.relative_coords_to(&location);
            //         info!("WORLDO POST: {:?}", transform.translation);
            //         // The transform.translation is the global transform since this is a top level entity
            //         last_transform_loc.0 = transform.translation;
            //     }
            // }
        } else {
            #[cfg(feature = "server")]
            {
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
}

//
// fn recursively_fix_locations(
//     entity: Entity,
//     parent_loc: Location,
//     mut parent_rotation: Quat,
//     q_info: &mut Query<(&mut Location, Option<&mut Transform>), Without<PlayerWorld>>,
//     q_children: &Query<&Children>,
//     commands: &mut Commands,
//     world: WorldWithin,
//     world_id: RapierContextEntityLink,
// ) {
//     let Ok((mut my_loc, my_trans)) = q_info.get_mut(entity) else {
//         return;
//     };
//
//     let translation = parent_rotation.inverse().mul_vec3(-my_loc.relative_coords_to(&parent_loc));
//
//     my_loc.last_transform_loc = Some(translation);
//
//     commands.entity(entity).insert((world, world_id));
//
//     if let Some(mut my_trans) = my_trans {
//         my_trans.translation = translation;
//
//         parent_rotation *= my_trans.rotation;
//     } else {
//         commands.entity(entity).insert((Transform::from_translation(translation),));
//     }
//
//     let Ok(children) = q_children.get(entity) else {
//         return;
//     };
//
//     let my_loc = *my_loc;
//
//     for &child in children {
//         recursively_fix_locations(child, my_loc, parent_rotation, q_info, q_children, commands, world, world_id);
//     }
// }

const WORLD_SWITCH_DISTANCE: f32 = SECTOR_DIMENSIONS / 2.0;
const WORLD_SWITCH_DISTANCE_SQRD: f32 = WORLD_SWITCH_DISTANCE * WORLD_SWITCH_DISTANCE;

fn create_physics_world(commands: &mut Commands) -> RapierContextEntityLink {
    let mut config = RapierConfiguration::new(1.0);
    config.gravity = Vec3::ZERO;
    let rw = commands.spawn((RapierContextSimulation::default(), config)).id();
    RapierContextEntityLink(rw)
}

fn move_players_between_worlds(
    players: Query<(Entity, &Location), (With<WorldWithin>, With<Anchor>)>,
    mut q_world_within: Query<(&mut WorldWithin, &mut RapierContextEntityLink)>,
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

                let (other_world_entity, other_body_world) = q_world_within.get(other_entity).map(|(ent, world)| (ent.0, *world)).unwrap();

                let (mut world_currently_in, mut body_world) = q_world_within.get_mut(entity).unwrap();

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

                let (mut world_within, mut body_world) = q_world_within.get_mut(entity).unwrap();

                world_within.0 = world_entity;
                *body_world = link;
            }
        }
    }
}

fn move_non_players_between_worlds(
    mut needs_world: Query<
        (Entity, &Location, Option<&mut WorldWithin>, Option<&mut RapierContextEntityLink>),
        (Without<Anchor>, Without<Parent>),
    >,
    players_with_worlds: Query<(&WorldWithin, &Location, &RapierContextEntityLink), With<Anchor>>,
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

/// Bevy's query mutability rules require this query to be in a separate system than fix_location
// fn find_need_fixed(
//     q_added_locations_no_parent: Query<Entity, (Added<Location>, Without<PlayerWorld>, Without<Parent>)>,
//     mut fix: ResMut<FixParentLocation>,
// ) {
//     for e in q_added_locations_no_parent.iter() {
//         fix.0.push(e);
//     }
// }

/// Handles any just-added locations that need to sync up to their transforms
// fn fix_location(
//     mut q_location_info: Query<(&mut Location, Option<&mut Transform>), Without<PlayerWorld>>,
//     q_children: Query<&Children>,
//     q_player_worlds: Query<(&Location, &WorldWithin, &RapierContextEntityLink), With<PlayerWorld>>,
//     mut commands: Commands,
//     q_player_world_loc: Query<&Location, With<PlayerWorld>>,
//     mut fix: ResMut<FixParentLocation>,
// ) {
//     for entity in std::mem::take(&mut fix.0) {
//         // This makes the borrow checker happy
//         let (location, _) = q_location_info.get(entity).expect("Guarenteed in query conditions");
//
//         let mut best_distance = None;
//         let mut best_world = None;
//         let mut best_world_id = None;
//
//         for (loc, ww, body_world) in q_player_worlds.iter() {
//             let distance = location.distance_sqrd(loc);
//
//             if best_distance.is_none() || distance < best_distance.unwrap() {
//                 best_distance = Some(distance);
//                 best_world = Some(*ww);
//                 best_world_id = Some(*body_world);
//             }
//         }
//
//         match (best_world, best_world_id) {
//             (Some(world), Some(world_id)) => {
//                 let Ok(loc) = q_player_world_loc.get(world.0) else {
//                     warn!("A player world was missing a location");
//                     continue;
//                 };
//
//                 recursively_fix_locations(
//                     entity,
//                     *loc,
//                     Quat::IDENTITY,
//                     &mut q_location_info,
//                     &q_children,
//                     &mut commands,
//                     world,
//                     world_id,
//                 );
//             }
//             _ => {
//                 warn!("Something was added with a location before a player world was created.");
//                 commands.entity(entity).log_components();
//             }
//         }
//     }
// }
//
// fn recursively_fix_locations(
//     entity: Entity,
//     parent_loc: Location,
//     mut parent_rotation: Quat,
//     q_info: &mut Query<(&mut Location, Option<&mut Transform>), Without<PlayerWorld>>,
//     q_children: &Query<&Children>,
//     commands: &mut Commands,
//     world: WorldWithin,
//     world_id: RapierContextEntityLink,
// ) {
//     let Ok((mut my_loc, my_trans)) = q_info.get_mut(entity) else {
//         return;
//     };
//
//     let translation = parent_rotation.inverse().mul_vec3(-my_loc.relative_coords_to(&parent_loc));
//
//     my_loc.last_transform_loc = Some(translation);
//
//     commands.entity(entity).insert((world, world_id));
//
//     if let Some(mut my_trans) = my_trans {
//         my_trans.translation = translation;
//
//         parent_rotation *= my_trans.rotation;
//     } else {
//         commands.entity(entity).insert((Transform::from_translation(translation),));
//     }
//
//     let Ok(children) = q_children.get(entity) else {
//         return;
//     };
//
//     let my_loc = *my_loc;
//
//     for &child in children {
//         recursively_fix_locations(child, my_loc, parent_rotation, q_info, q_children, commands, world, world_id);
//     }
// }

type TransformLocationQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Name,
        &'static WorldWithin,
        &'static GlobalTransform,
        &'static mut Location,
        Option<&'static mut Transform>,
        Option<&'static PreviousLocation>,
        Option<&'static LastTransformTranslation>,
        Option<&'static SetTransformBasedOnLocationFlag>,
    ),
    Without<PlayerWorld>,
>;

/// This system syncs the locations up with their changes in transforms.
fn sync_transforms_and_locations(
    q_entities: Query<(Entity, &WorldWithin), (Without<PlayerWorld>, With<Transform>, With<Location>, Without<Parent>)>,
    q_loc: Query<&Location, With<PlayerWorld>>,
    mut q_data: TransformLocationQuery,
    q_pw: Query<&Location, With<PlayerWorld>>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    for (entity, world_within) in q_entities.iter() {
        let Ok(pw_loc) = q_loc.get(world_within.0) else {
            continue;
        };

        recursively_sync_transforms_and_locations(
            *pw_loc,
            Vec3::ZERO,
            Quat::IDENTITY,
            entity,
            &mut commands,
            &mut q_data,
            &q_children,
            &q_pw,
        );
    }
}

#[derive(Component)]
struct PreviousLocation(Location);

fn recursively_sync_transforms_and_locations(
    parent_loc: Location,
    parent_g_trans: Vec3,
    parent_g_rot: Quat,
    parent_delta_g_trans: Vec3,
    ent: Entity,
    commands: &mut Commands,
    q_data: &mut TransformLocationQuery,
    q_children: &Query<&Children>,
    q_pw: &Query<&Location, With<PlayerWorld>>,
) {
    let Ok((name, world_within, _, mut my_loc, my_transform, my_prev_loc, last_trans_trans, set_trans)) = q_data.get_mut(ent) else {
        return;
    };

    let Ok(world_loc) = q_pw.get(world_within.0) else {
        return;
    };

    info!("DOING {}", name);
    let (local_translation, local_rotation) = if let Some(mut my_transform) = my_transform {
        let mut delta_g_trans = Vec3::ZERO;

        if set_trans.is_some() {
            info!("SETTING TRANS! - Was {}", my_transform.translation);
            my_transform.translation = parent_g_rot.inverse() * ((*my_loc - parent_loc).absolute_coords_f32());
            info!("SETTING TRANS! - NOW {}", my_transform.translation);
        } else {
            let g_trans = parent_g_trans + (parent_g_rot * my_transform.translation);

            // Calculates the change in location since the last time this ran
            // let delta_loc = (*my_loc - my_prev_loc.map(|x| x.0).unwrap_or(*my_loc)).absolute_coords_f32();
            // info!("G TRANS: {}", g_trans);
            delta_g_trans = last_trans_trans.map(|x| g_trans - x.0).unwrap_or(Vec3::ZERO);

            let my_global_location_trans = (*my_loc - *world_loc).absolute_coords_f32();
            let my_local_location_trans = parent_g_rot.inverse() * (my_global_location_trans - parent_g_trans);
            let my_delta_local_trans = delta_g_trans - parent_delta_g_trans;
            // info!("Delta loc: {delta_loc:?}");
            // info!("Delta G trans: {delta_g_trans:?}");
            //
            info!(
                "PRE: {my_loc:?} | TRANS: {} | my_local_location_trans: {my_local_location_trans:?}",
                my_transform.translation
            );

            // Applies that change to the transform
            my_transform.translation += parent_g_rot.inverse() * delta_loc;

            // Updates the location to be based on the parent's location + your absolute coordinates to your parent.
            *my_loc = parent_loc + parent_g_rot * (my_transform.translation + delta_g_trans);
            info!("POST: {my_loc:?} | TRANS: {}", my_transform.translation);
        }

        (my_transform.translation, my_transform.rotation, delta_g_trans)
    } else {
        info!("SETTING TRANS the odd way!!!!!!");
        let translation = parent_g_rot.inverse() * ((*my_loc - parent_loc).absolute_coords_f32());

        commands.entity(ent).insert(Transform::from_translation(translation));

        info!("SETTING TRANS the odd way!!!!!! - is {}", translation);

        (translation, Quat::IDENTITY, Vec3::ZERO)
    };

    let my_g_trans = parent_g_trans + parent_g_rot * local_translation;
    let ltt = LastTransformTranslation(my_g_trans);
    commands
        .entity(ent)
        .insert((ltt, PreviousLocation(*my_loc)))
        .remove::<SetTransformBasedOnLocationFlag>();

    let my_loc = *my_loc;
    let my_g_trans = my_g_trans;
    let my_g_rot = parent_g_rot.mul_quat(local_rotation);

    if let Ok(children) = q_children.get(ent) {
        for &child in children.iter() {
            recursively_sync_transforms_and_locations(my_loc, my_g_trans, my_g_rot, child, commands, q_data, q_children, q_pw);
        }
    }
}

/// Handles children and their locations.
// pub fn handle_child_syncing(
//     q_top_level: Query<(&Location, &Transform, &Children), Without<Parent>>,
//     mut q_data: Query<(&mut Location, &mut Transform, &PreviousLocation), With<Parent>>,
//     q_children: Query<&Children>,
// ) {
//     for (loc, trans, children) in q_top_level.iter() {
//         for &child in children.iter() {
//             recursively_sync_child(*loc, trans.translation, trans.rotation, child, &mut q_data, &q_children);
//         }
//     }
// }

/// Fixes oddities that happen when changing parent of player
// fn player_changed_parent(
//     q_parent: Query<(&GlobalTransform, &Location)>,
//     mut q_local_player: Query<(Entity, &mut Transform, &Location, &Parent, &PlayerWorld), (Changed<Parent>, With<Anchor>)>,
// ) {
//     for (entity, mut player_trans, player_loc, parent, player_world) in q_local_player.iter_mut() {
//         if entity != player_world.player {
//             // This problem only effects players that control their world
//             continue;
//         }
//
//         let Ok((parent_trans, parent_loc)) = q_parent.get(parent.get()) else {
//             continue;
//         };
//
//         // Because the player's translation is always 0, 0, 0 we need to adjust it so the player is put into the
//         // right spot in its parent.
//         player_trans.translation = Quat::from_affine3(&parent_trans.affine())
//             .inverse()
//             .mul_vec3((*player_loc - *parent_loc).absolute_coords_f32());
//     }
// }
//
// fn create_default_world_within(
//     mut commands: Commands,
//     q_rapier_context: Query<Entity, With<DefaultRapierContext>>) {
//     let context_ent = q_rapier_context.single();
//     commands.spawn(PlayerWorld)
// }
//
// fn client_update_worlds(
//     q_needs_world: Query<Entity, (With<Location>, Without<WorldWithin>)>,
//     pw: Query<Entity, With<PlayerWorld>>,
//     mut commands: Commands,
// ) {
//     let Ok(pw) = pw.get_single() else {
//         return;
//     };
//     for entity in q_needs_world.iter() {
//         commands.entity(entity).insert(WorldWithin(pw));
//     }
// }

pub(super) fn register(app: &mut App) {
    /*
    *Between->>apply_set_position:
    apply_set_position->>handle_worlds
    handle_worlds->>sync_transforms_and_locations:
    sync_transforms_and_locations->>handle_child_syncing:
    handle_child_syncing->>add_previous_location:
    add_previous_location->>Between:

    * */
    app.add_systems(
        Update,
        (
            propagate_transforms, // TODO: Maybe not this?
            apply_set_position,
            ensure_worlds_have_anchors,
            #[cfg(feature = "server")]
            (move_players_between_worlds, move_non_players_between_worlds, remove_empty_worlds).chain(),
            sync_transforms_and_locations,
            // set_transform_based_on_location,
            propagate_transforms, // TODO: Maybe not this? Idk
        )
            .chain()
            .in_set(LocationPhysicsSet::DoPhysics)
            // .in_set(CosmosBundleSet::HandleCosmosBundles)
            .in_set(NetworkingSystemsSet::Between),
    );
}
