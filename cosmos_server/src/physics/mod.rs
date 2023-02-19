use bevy::prelude::*;
use bevy_rapier3d::prelude::{RapierContext, RapierWorld};
use cosmos_core::{
    entities::player::Player,
    physics::{
        location::{Location, SECTOR_DIMENSIONS},
        player_world::{PlayerWorld, WorldWithin},
    },
};

use crate::state::GameState;

const WORLD_SWITCH_DISTANCE: f32 = SECTOR_DIMENSIONS / 2.0;
const WORLD_SWITCH_DISTANCE_SQRD: f32 = WORLD_SWITCH_DISTANCE * WORLD_SWITCH_DISTANCE;

pub fn assign_player_world(
    player_worlds: &Query<(&Location, &WorldWithin), (With<Player>, Without<Parent>)>,
    player_entity: Entity,
    location: &Location,
    commands: &mut Commands,
    rapier_context: &mut RapierContext,
) {
    let mut best_distance = None;
    let mut best_world = None;

    for (loc, ww) in player_worlds.iter() {
        let distance = location.distance_sqrd(loc);

        if distance <= WORLD_SWITCH_DISTANCE
            && (best_distance.is_none() || distance < best_distance.unwrap())
        {
            best_distance = Some(distance);
            best_world = Some(*ww);
        }
    }

    if let Some(world) = best_world {
        commands.entity(player_entity).insert(world);
    } else {
        let world_id = rapier_context.add_world(RapierWorld::default());
        let world_entity = commands
            .spawn((
                PlayerWorld {
                    player: player_entity,
                    world_id,
                },
                *location,
            ))
            .id();

        commands
            .entity(player_entity)
            .insert(WorldWithin(world_entity));
    }
}

fn move_players_between_worlds(
    players: Query<(Entity, &Location), (With<WorldWithin>, With<Player>)>,
    mut world_within_query: Query<&mut WorldWithin>,

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

                let world_entity = world_within_query.get(other_entity).unwrap().0;
                let mut world_currently_in = world_within_query.get_mut(entity).unwrap();

                let distance = location.distance_sqrd(other_location);

                // println!("Distance: {distance} vs {WORLD_SWITCH_DISTANCE_SQRD}");

                if distance < WORLD_SWITCH_DISTANCE_SQRD {
                    if world_currently_in.0 != world_entity {
                        world_currently_in.0 = world_entity;
                        println!("Swapped to other player's world!");

                        needs_new_world = false;
                        changed = true;
                    }
                    break;
                } else {
                    if world_currently_in.0 == world_entity {
                        needs_new_world = true;
                    }
                }
            }

            if needs_new_world {
                getting_new_world.push(entity);

                let world_id = rapier_context.add_world(RapierWorld::default());

                let world_entity = commands
                    .spawn((
                        PlayerWorld {
                            player: entity,
                            world_id,
                        },
                        *location,
                    ))
                    .id();

                world_within_query.get_mut(entity).unwrap().0 = world_entity;

                println!("BOOM! NEW WORLD CREATED!");
            }
        }
    }
}

fn monitor_without_worlds(
    needs_world: Query<
        (Entity, &Location),
        (Without<WorldWithin>, Without<Player>, Without<Parent>),
    >,
    players_with_worlds: Query<(&WorldWithin, &Location), With<Player>>,
    mut commands: Commands,
) {
    for (entity, location) in needs_world.iter() {
        let mut best_ww = None;
        let mut best_dist = None;

        for (ww, player_loc) in players_with_worlds.iter() {
            let dist = player_loc.distance_sqrd(location);

            if best_ww.is_none() || dist < best_dist.unwrap() {
                best_ww = Some(*ww);
                best_dist = Some(dist);
            }
        }

        if let Some(ww) = best_ww {
            commands.entity(entity).insert(ww);
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system_set(
        SystemSet::on_update(GameState::Playing)
            .with_system(monitor_without_worlds)
            .with_system(move_players_between_worlds),
    );
}
