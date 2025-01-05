//!
//!
//! - Apply set position
//! - Set location based on transform
//!     This can only be done for children - doesn't make sense for parents
//! - Update Worlds
//! - sync transforms and locations
//!
//!

use bevy::{
    prelude::*,
    transform::systems::{propagate_transforms, sync_simple_transforms},
    utils::HashSet,
};
use bevy_rapier3d::{
    plugin::{RapierConfiguration, RapierContextEntityLink},
    prelude::RapierContextSimulation,
};

use crate::{
    netty::system_sets::NetworkingSystemsSet,
    physics::{
        location::LastTransformTranslation,
        player_world::{PlayerWorld, WorldWithin},
    },
};

use super::{Location, LocationPhysicsSet, Sector, SetPosition, SECTOR_DIMENSIONS};
#[cfg(doc)]
use crate::netty::client::LocalPlayer;

#[derive(Component)]
/// Anything that has this component will be treated as an anchor for the [`PlayerWorld`]s.
///
/// If the [`Anchor`] entity is a child of another, its parent will be treated as the anchor.
///
/// ## Client
/// This should only be put on the [`LocalPlayer`].
///
/// Putting this on anything else will probably cause issues, since the client can only ever have
/// one world.
///
/// ## Server
/// This can be put on any entity with a [`Location`] and a [`Transform`]. This will act as a focal
/// point for a [`PlayerWorld`].
pub struct Anchor;

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

fn ensure_worlds_have_anchors(
    q_loc_no_parent: Query<&Location, (Without<PlayerWorld>, Without<Parent>)>,
    mut q_everything: Query<(&mut LastTransformTranslation, &mut Transform, &WorldWithin, Option<&Parent>), With<Location>>,
    trans_query_with_parent: Query<&Location, (Without<PlayerWorld>, With<Parent>)>,
    q_anchors: Query<(&WorldWithin, Entity), With<Anchor>>,
    #[cfg(feature = "server")] everything_query: Query<(&WorldWithin, Entity)>,
    parent_query: Query<&Parent>,
    entity_query: Query<Entity>,
    mut world_query: Query<(Entity, &mut PlayerWorld, &mut Location)>,

    mut commands: Commands,
) {
    for (world_entity, mut world, mut world_location) in world_query.iter_mut() {
        if let Ok(mut player_entity) = entity_query.get(world.player) {
            while let Ok(parent) = parent_query.get(player_entity) {
                let parent_entity = parent.get();
                if q_loc_no_parent.contains(parent_entity) {
                    player_entity = parent.get();
                } else {
                    break;
                }
            }

            let Ok(location) = q_loc_no_parent
                .get(player_entity)
                .or_else(|_| match trans_query_with_parent.get(player_entity) {
                    Ok(loc) => Ok(loc),
                    Err(x) => Err(x),
                })
            else {
                // The player was just added & doesn't have a transform yet - only a location.
                continue;
            };

            let delta = (*location - *world_location).absolute_coords_f32();
            *world_location = *location;

            for (mut ltt, mut t, _, parent) in q_everything.iter_mut().filter(|(_, _, ww, _)| ww.0 == world_entity) {
                ltt.0 -= delta;
                if parent.is_none() {
                    t.translation -= delta;
                }
            }
        } else {
            #[cfg(feature = "server")]
            {
                // The player has disconnected
                // Either: Find a new player to have the world
                // Or: Move everything over to the closest world by removing the WorldWithin component

                let mut found = false;
                // find player
                for (world_within, entity) in q_anchors.iter() {
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

const WORLD_SWITCH_DISTANCE: f32 = SECTOR_DIMENSIONS / 2.0;
const WORLD_SWITCH_DISTANCE_SQRD: f32 = WORLD_SWITCH_DISTANCE * WORLD_SWITCH_DISTANCE;

fn create_physics_world(commands: &mut Commands) -> RapierContextEntityLink {
    let mut config = RapierConfiguration::new(1.0);
    config.gravity = Vec3::ZERO;
    let rw = commands.spawn((RapierContextSimulation::default(), config)).id();
    RapierContextEntityLink(rw)
}

#[cfg(feature = "server")]
fn move_anchors_between_worlds(
    q_anchors: Query<(Entity, &Location), (With<WorldWithin>, With<Anchor>)>,
    mut q_world_within: Query<(&mut WorldWithin, &mut RapierContextEntityLink)>,
    mut commands: Commands,
) {
    let mut changed = true;

    let mut getting_new_world = Vec::new();

    while changed {
        changed = false;

        for (entity, location) in q_anchors.iter() {
            let mut needs_new_world = false;

            for (other_entity, other_location) in q_anchors.iter() {
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

#[cfg(feature = "server")]
fn move_non_anchors_between_worlds(
    mut needs_world: Query<
        (Entity, &Location, Option<&mut WorldWithin>, Option<&mut RapierContextEntityLink>),
        (Without<Anchor>, Without<Parent>),
    >,
    anchors_with_worlds: Query<(&WorldWithin, &Location, &RapierContextEntityLink), With<Anchor>>,
    mut commands: Commands,
) {
    for (entity, location, maybe_within, maybe_body_world) in needs_world.iter_mut() {
        let mut best_ww = None;
        let mut best_dist = None;
        let mut best_world_id = None;

        for (ww, player_loc, body_world) in anchors_with_worlds.iter() {
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
#[cfg(feature = "server")]
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

type TransformLocationQuery<'w, 's> = Query<
    'w,
    's,
    (
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
    q_entities: Query<(Entity, &WorldWithin), (Without<PlayerWorld>, With<Location>, Without<Parent>)>,
    q_loc: Query<&Location, With<PlayerWorld>>,
    mut q_data: TransformLocationQuery,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    for (entity, world_within) in q_entities.iter() {
        let Ok(pw_loc) = q_loc.get(world_within.0) else {
            continue;
        };

        recursively_sync_transforms_and_locations(
            *pw_loc,
            *pw_loc,
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ZERO,
            entity,
            &mut commands,
            &mut q_data,
            &q_children,
        );
    }
}

#[derive(Component)]
struct PreviousLocation(Location);

fn recursively_sync_transforms_and_locations(
    parent_loc: Location,
    parent_prev_loc: Location,
    parent_g_trans: Vec3,
    parent_g_rot: Quat,
    parent_delta_g_trans: Vec3,
    ent: Entity,
    commands: &mut Commands,
    q_data: &mut TransformLocationQuery,
    q_children: &Query<&Children>,
) {
    let Ok((mut my_loc, my_transform, my_prev_loc, last_trans_trans, set_trans)) = q_data.get_mut(ent) else {
        return;
    };

    let parent_g_rot = parent_g_rot.normalize();

    let (local_translation, local_rotation, delta_g_trans) = if let Some(mut my_transform) = my_transform {
        let mut delta_g_trans = Vec3::ZERO;

        if set_trans.is_some() {
            my_transform.translation = parent_g_rot.inverse().normalize() * ((*my_loc - parent_loc).absolute_coords_f32());
        } else {
            let g_trans = parent_g_trans + (parent_g_rot * my_transform.translation);

            let parent_delta_loc = parent_loc - parent_prev_loc;
            delta_g_trans = last_trans_trans.map(|x| g_trans - x.0).unwrap_or(Vec3::ZERO);

            let delta_local_trans = delta_g_trans - parent_delta_g_trans;

            // WARNING: THIS COULD BLOW UP if the delta loc is huge in f32 coords. Idk how to do this better
            // though.
            let delta_loc = my_prev_loc.map(|x| (*my_loc - x.0).absolute_coords_f32()).unwrap_or(Vec3::ZERO);
            *my_loc = *my_loc + delta_local_trans + parent_delta_loc;
            let my_local_rotated_trans = (*my_loc - parent_loc).absolute_coords_f32();
            let my_local_location_based_trans = parent_g_rot.inverse().normalize() * my_local_rotated_trans;

            my_transform.translation = my_local_location_based_trans;

            // Changes in location mess up the children.
            delta_g_trans += delta_loc;
        }

        (my_transform.translation, my_transform.rotation, delta_g_trans)
    } else {
        let translation = parent_g_rot.inverse() * ((*my_loc - parent_loc).absolute_coords_f32());

        commands.entity(ent).insert(Transform::from_translation(translation));

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
    let my_prev_loc = my_prev_loc.map(|x| x.0).unwrap_or(my_loc);

    if let Ok(children) = q_children.get(ent) {
        for &child in children.iter() {
            recursively_sync_transforms_and_locations(
                my_loc,
                my_prev_loc,
                my_g_trans,
                my_g_rot,
                delta_g_trans,
                child,
                commands,
                q_data,
                q_children,
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            (sync_simple_transforms, propagate_transforms).chain(), // TODO: Maybe not this?
            apply_set_position,
            ensure_worlds_have_anchors,
            #[cfg(feature = "server")]
            (move_anchors_between_worlds, move_non_anchors_between_worlds, remove_empty_worlds).chain(),
            sync_transforms_and_locations,
            // set_transform_based_on_location,
        )
            .chain()
            .in_set(LocationPhysicsSet::DoPhysics)
            // .in_set(CosmosBundleSet::HandleCosmosBundles)
            .in_set(NetworkingSystemsSet::Between),
    );
}
