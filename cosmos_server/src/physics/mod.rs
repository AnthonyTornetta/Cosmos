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
        location::{CosmosBundleSet, Location, LocationPhysicsSet, SECTOR_DIMENSIONS},
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

pub(super) fn register(app: &mut App) {
    collider_disabling::register(app);

    app.configure_sets(Update, LocationPhysicsSet::DoPhysics)
        // .add_systems(
        // Update,
        // (), // .chain()
        // .after(CosmosBundleSet::HandleCosmosBundles)
        // .in_set(LocationPhysicsSet::DoPhysics)
        // .run_if(in_state(GameState::Playing))
        // .before(NetworkingSystemsSet::ReceiveMessages),
        // )
        // .add_systems(PostUpdate, fix_location)
        // This must be last due to commands being delayed when adding PhysicsWorlds.
        .add_systems(
            Last,
            (move_players_between_worlds, move_non_players_between_worlds, remove_empty_worlds).chain(),
        )
        .init_resource::<FixParentLocation>();
}
