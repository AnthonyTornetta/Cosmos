//! Handles the systems that sync up worlds, locations, and transforms

use bevy::{
    ecs::component::StorageType,
    prelude::*,
    transform::systems::{propagate_transforms, sync_simple_transforms},
};

#[cfg(feature = "server")]
use bevy::utils::HashSet;

#[cfg(feature = "server")]
use bevy_rapier3d::{
    plugin::{RapierConfiguration, RapierContextEntityLink},
    prelude::RapierContextSimulation,
};

use crate::{
    netty::system_sets::NetworkingSystemsSet,
    physics::player_world::{PlayerWorld, WorldWithin},
};

#[cfg(feature = "server")]
use super::SECTOR_DIMENSIONS;
use super::{Location, LocationPhysicsSet, SetPosition};

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

fn loc_from_trans(
    entity: Entity,
    q_trans: &Query<&Transform>,
    q_x: &Query<(Entity, Option<&Location>, Option<&SetPosition>)>,
    q_g_trans: &Query<&GlobalTransform>,
    q_parent: &Query<&Parent>,
) -> Option<Location> {
    let (entity, loc, set_pos) = q_x.get(entity).expect("Invalid entity given");

    match set_pos {
        None | Some(SetPosition::Transform) => loc.copied(),
        Some(SetPosition::Location) => {
            if let Ok(p) = q_parent.get(entity) {
                let parent_g_trans = q_g_trans.get(p.get()).ok()?;
                let my_trans = q_trans.get(entity).ok()?;

                loc_from_trans(p.get(), q_trans, q_x, q_g_trans, q_parent)
                    .map(|x| x + (parent_g_trans.rotation().inverse() * my_trans.translation))
            } else {
                warn!("Location set based solely on global transform - you probably didn't mean to do this.");
                q_g_trans
                    .get(entity)
                    .ok()
                    .map(|g_trans| Location::new(g_trans.translation(), Default::default()))
            }
        }
    }
}

#[derive(Component)]
struct SetTransformBasedOnLocationFlag;

fn apply_set_position_single(
    ent: In<Entity>,
    q_set_position: Query<&SetPosition>,
    q_x: Query<(Entity, Option<&Location>, Option<&SetPosition>)>,
    q_trans: Query<&Transform>,
    q_parent: Query<&Parent>,
    q_g_trans: Query<&GlobalTransform>,
    mut commands: Commands,
) {
    const DEFAULT_SET_POS: SetPosition = SetPosition::Transform;

    let entity = ent.0;

    let set_pos = q_set_position.get(entity).unwrap_or(&DEFAULT_SET_POS);
    match set_pos {
        SetPosition::Transform => {
            let mut ecmds = commands.entity(entity);
            ecmds.insert(SetTransformBasedOnLocationFlag).remove::<SetPosition>();
        }
        SetPosition::Location => {
            if let Some(loc_from_trans) = loc_from_trans(entity, &q_trans, &q_x, &q_g_trans, &q_parent) {
                commands
                    .entity(entity)
                    .insert((loc_from_trans, PreviousLocation(loc_from_trans)))
                    .remove::<SetPosition>();
            }
        }
    }
}

fn apply_set_position(
    q_location_added: Query<Entity, (Without<SetPosition>, Added<Location>)>,
    q_set_position: Query<(Entity, &SetPosition)>,
    q_x: Query<(Entity, Option<&Location>, Option<&SetPosition>)>,
    q_trans: Query<&Transform>,
    q_parent: Query<&Parent>,
    q_g_trans: Query<&GlobalTransform>,
    mut commands: Commands,
) {
    const DEFAULT_SET_POS: SetPosition = SetPosition::Transform;

    for (entity, set_pos) in q_location_added
        .iter()
        .map(|ent| (ent, &DEFAULT_SET_POS))
        .chain(q_set_position.iter())
    {
        match set_pos {
            SetPosition::Transform => {
                let mut ecmds = commands.entity(entity);
                ecmds.insert(SetTransformBasedOnLocationFlag).remove::<SetPosition>();
            }
            SetPosition::Location => {
                if let Some(loc_from_trans) = loc_from_trans(entity, &q_trans, &q_x, &q_g_trans, &q_parent) {
                    commands
                        .entity(entity)
                        .insert((loc_from_trans, PreviousLocation(loc_from_trans)))
                        .remove::<SetPosition>();
                }
            }
        }
    }
}

fn reposition_worlds_around_anchors(
    q_loc_no_parent: Query<&Location, (Without<PlayerWorld>, Without<Parent>)>,
    mut q_trans_no_parent: Query<(&mut Transform, &WorldWithin), (Without<Parent>, With<Location>)>,
    trans_query_with_parent: Query<&Location, (Without<PlayerWorld>, With<Parent>)>,
    #[cfg(feature = "server")] q_anchors: Query<(&WorldWithin, Entity), With<Anchor>>,
    #[cfg(feature = "server")] everything_query: Query<(&WorldWithin, Entity)>,
    parent_query: Query<&Parent>,
    entity_query: Query<Entity>,
    mut world_query: Query<(Entity, &mut PlayerWorld, &mut Location)>,
    #[cfg(feature = "server")] mut commands: Commands,
) {
    #[allow(unused_mut)] // the server needs `world` to be mut, the client doesn't.
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
                .or_else(|_| trans_query_with_parent.get(player_entity))
            else {
                // The player was just added & doesn't have a transform yet - only a location.
                continue;
            };

            let delta = (*location - *world_location).absolute_coords_f32();
            *world_location = *location;

            for (mut t, _) in q_trans_no_parent.iter_mut().filter(|(_, ww)| ww.0 == world_entity) {
                t.translation -= delta;
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

#[cfg(feature = "server")]
const WORLD_SWITCH_DISTANCE: f32 = SECTOR_DIMENSIONS / 2.0;
#[cfg(feature = "server")]
const WORLD_SWITCH_DISTANCE_SQRD: f32 = WORLD_SWITCH_DISTANCE * WORLD_SWITCH_DISTANCE;

#[cfg(feature = "server")]
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
fn move_non_anchors_between_worlds_single(
    ent: In<Entity>,
    mut needs_world: Query<(&Location, Option<&mut WorldWithin>, Option<&mut RapierContextEntityLink>), (Without<Anchor>, Without<Parent>)>,
    anchors_with_worlds: Query<(&WorldWithin, &Location, &RapierContextEntityLink), With<Anchor>>,
    mut commands: Commands,
) {
    let entity = ent.0;

    let Ok((location, maybe_within, maybe_body_world)) = needs_world.get_mut(entity) else {
        return;
    };

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
        Option<&'static SetTransformBasedOnLocationFlag>,
    ),
    Without<PlayerWorld>,
>;

/// This system syncs the locations up with their changes in transforms.
fn sync_transforms_and_locations_single(
    ent: In<Entity>,
    // for now this only supports stuff w/out parents. That's fine for now
    q_entities: Query<&WorldWithin, (Without<PlayerWorld>, With<Location>, Without<Parent>)>,
    q_loc: Query<&Location, With<PlayerWorld>>,
    mut q_data: TransformLocationQuery,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    let entity = ent.0;
    if let Ok(world_within) = q_entities.get(entity) {
        let Ok(pw_loc) = q_loc.get(world_within.0) else {
            return;
        };

        recursively_sync_transforms_and_locations(*pw_loc, Vec3::ZERO, Quat::IDENTITY, entity, &mut commands, &mut q_data, &q_children);
    }
}

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

        recursively_sync_transforms_and_locations(*pw_loc, Vec3::ZERO, Quat::IDENTITY, entity, &mut commands, &mut q_data, &q_children);
    }
}

#[derive(Component)]
struct PreviousLocation(Location);

fn recursively_sync_transforms_and_locations(
    parent_loc: Location,
    parent_g_trans: Vec3,
    parent_g_rot: Quat,
    ent: Entity,
    commands: &mut Commands,
    q_data: &mut TransformLocationQuery,
    q_children: &Query<&Children>,
) {
    let Ok((mut my_loc, my_transform, my_prev_loc, set_trans)) = q_data.get_mut(ent) else {
        return;
    };

    let parent_g_rot = parent_g_rot.normalize();

    let (local_translation, local_rotation) = if let Some(mut my_transform) = my_transform {
        if set_trans.is_some() {
            my_transform.translation = parent_g_rot.inverse().normalize() * ((*my_loc - parent_loc).absolute_coords_f32());
        } else {
            // Calculates the change in location since the last time this ran
            // WARNING: THIS COULD BLOW UP if the delta loc is huge in f32 coords. Idk how to do this better
            let delta_loc = (*my_loc - my_prev_loc.map(|x| x.0).unwrap_or(*my_loc)).absolute_coords_f32();

            // Applies that change to the transform
            let delta = parent_g_rot.inverse().mul_vec3(delta_loc);
            if delta != Vec3::ZERO {
                my_transform.translation += delta;
            }

            // Calculates how far away the entity was from its parent + its delta location.
            let transform_delta_parent = parent_g_rot * my_transform.translation;
            let new_loc = parent_loc + transform_delta_parent;
            if *my_loc != new_loc {
                *my_loc = new_loc;
            }
        }

        (my_transform.translation, my_transform.rotation)
    } else {
        let translation = parent_g_rot.inverse() * ((*my_loc - parent_loc).absolute_coords_f32());

        commands.entity(ent).insert(Transform::from_translation(translation));

        (translation, Quat::IDENTITY)
    };

    let my_g_trans = parent_g_trans + parent_g_rot * local_translation;
    commands
        .entity(ent)
        .insert(PreviousLocation(*my_loc))
        .remove::<SetTransformBasedOnLocationFlag>();

    let my_loc = *my_loc;
    let my_g_rot = parent_g_rot * local_rotation;
    let my_prev_loc = my_prev_loc.map(|x| x.0).unwrap_or(my_loc);

    if let Ok(children) = q_children.get(ent) {
        for &child in children.iter() {
            recursively_sync_transforms_and_locations(my_prev_loc, my_g_trans, my_g_rot, child, commands, q_data, q_children);
        }
    }
}

#[cfg(feature = "client")]
fn assign_everything_client_world_single(
    ent: In<Entity>,
    mut commands: Commands,
    q_player_world: Query<Entity, With<PlayerWorld>>,
    q_loc_no_world: Query<(), (With<Location>, Without<WorldWithin>, Without<PlayerWorld>)>,
) {
    let entity = ent.0;

    if q_loc_no_world.contains(entity) {
        let Ok(pw) = q_player_world.get_single() else {
            return;
        };
        commands.entity(entity).insert(WorldWithin(pw));
    }
}

#[cfg(feature = "client")]
fn assign_everything_client_world(
    mut commands: Commands,
    q_player_world: Query<Entity, With<PlayerWorld>>,
    q_loc_no_world: Query<Entity, (With<Location>, Without<WorldWithin>, Without<PlayerWorld>)>,
) {
    for ent in q_loc_no_world.iter() {
        let Ok(pw) = q_player_world.get_single() else {
            continue;
        };
        commands.entity(ent).insert(WorldWithin(pw));
    }
}

impl Component for Location {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut bevy::ecs::component::ComponentHooks) {
        // A lot of times you want to add something to the world after the [`LocationPhysicsSet::DoPhysics`] set,
        // so this will allow you to do that without messing up any positioning logic.
        hooks.on_add(|mut world, entity, _component_id| {
            if !world.contains_resource::<DoPhysicsDone>() {
                // Don't do all this if it's going to happen later this frame.
                // This prevents a lot of unneeded work from happening
                return;
            }
            let [ent] = world.entity(&[entity]);
            if ent.contains::<Anchor>() {
                return;
            }

            let mut cmds = world.commands();
            cmds.run_system_cached_with(apply_set_position_single, entity);
            #[cfg(feature = "server")]
            cmds.run_system_cached_with(move_non_anchors_between_worlds_single, entity);
            #[cfg(feature = "client")]
            cmds.run_system_cached_with(assign_everything_client_world_single, entity);
            cmds.run_system_cached_with(sync_transforms_and_locations_single, entity);
        });
    }
}

#[derive(Resource)]
struct DoPhysicsDone;

fn remove_do_physics_done(mut commands: Commands) {
    commands.remove_resource::<DoPhysicsDone>();
}

fn do_physics_done(mut commands: Commands) {
    commands.insert_resource(DoPhysicsDone);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            (sync_simple_transforms, propagate_transforms).chain(), // TODO: Maybe not this?
            apply_set_position,
            reposition_worlds_around_anchors,
            #[cfg(feature = "server")]
            (move_anchors_between_worlds, move_non_anchors_between_worlds, remove_empty_worlds).chain(),
            #[cfg(feature = "client")]
            assign_everything_client_world,
            sync_transforms_and_locations,
            do_physics_done,
        )
            .chain()
            .in_set(LocationPhysicsSet::DoPhysics)
            // .in_set(CosmosBundleSet::HandleCosmosBundles)
            .in_set(NetworkingSystemsSet::Between),
    )
    .add_systems(PostUpdate, remove_do_physics_done);
}
