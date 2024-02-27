use bevy::{
    app::{App, Update},
    ecs::{
        component::Component,
        entity::Entity,
        query::{Added, Or, With, Without},
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res},
    },
    hierarchy::BuildChildren,
    math::Vec3,
    time::Time,
    transform::components::Transform,
};
use bevy_rapier3d::dynamics::Velocity;
use cosmos_core::{
    entities::player::Player,
    physics::location::Location,
    structure::{
        shared::DespawnWithStructure,
        ship::{pilot::Pilot, ship_movement::ShipMovement, Ship},
        systems::{laser_cannon_system::LaserCannonSystem, SystemActive, Systems},
    },
};

use crate::{
    persistence::loading::LoadingSystemSet, structure::systems::laser_cannon_system::LASER_BASE_VELOCITY,
    universe::spawners::pirate::Pirate,
};

use super::AiControlled;

#[derive(Component)]
pub struct PirateTarget;

#[derive(Component, Default)]
struct PirateAi {
    accel_per_sec: Option<f32>,
}

const PIRATE_MAX_CHASE_DISTANCE: f32 = 20_000.0;

/// Attempt to maintain a distance of ~500 blocks from closest target
fn handle_pirate_movement(
    mut commands: Commands,
    q_laser_cannon_system: Query<Entity, With<LaserCannonSystem>>,
    mut q_pirates: Query<(&Systems, &Location, Velocity, &mut ShipMovement, &mut Transform, &PirateAi), With<Pirate>>,
    q_players: Query<(&Location, &Velocity), (Without<Pirate>, With<PirateTarget>)>,
) {
    for (pirate_systems, pirate_loc, pirate_vel, mut pirate_ship_movement, mut pirate_transform, pirate_ai) in q_pirates.iter_mut() {
        let Some(accel_per_sec) = pirate_ai.accel_per_sec else {
            continue;
        };

        let Some((target_loc, target_vel)) = q_players
            .iter()
            .filter(|x| x.0.is_within_reasonable_range(pirate_loc))
            .min_by_key(|x| x.0.distance_sqrd(pirate_loc).floor() as u64)
        else {
            continue;
        };

        let dist = target_loc.distance_sqrd(pirate_loc).sqrt();

        if dist > PIRATE_MAX_CHASE_DISTANCE {
            continue;
        }

        let laser_vel = pirate_vel.linvel.length() + LASER_BASE_VELOCITY - target_vel.linvel.length();

        let distance = (*target_loc - *pirate_loc).absolute_coords_f32();
        let laser_secs_to_reach_target = (distance.length() / laser_vel).max(0.0);

        let direction = (distance + (target_vel.linvel - pirate_vel.linvel) * laser_secs_to_reach_target).normalize_or_zero();

        // I don't feel like doing the angle math to make it use angular acceleration to look towards it.
        pirate_transform.look_to(direction, Vec3::Y);

        // LASER_BASE_VELOCITY

        // let target_net_v = target_vel.linvel - pirate_vel.linvel;

        // let delta_v = -(-direction - (target_net_v - pirate_vel.linvel)).normalize_or_zero();

        // pirate_vel.linvel = delta_v * 128.0;
        // // pirate_ship_movement

        if dist > 500.0 {
            pirate_ship_movement.movement = Vec3::Z;
        } else {
            pirate_ship_movement.movement = -Vec3::Z;
        }

        if let Ok(laser_cannon_system) = pirate_systems.query(&q_laser_cannon_system) {
            if dist > 2000.0 {
                commands.entity(laser_cannon_system).remove::<SystemActive>();
            } else {
                commands.entity(laser_cannon_system).insert(SystemActive);
            }
        }
    }
}

fn add_pirate_targets(mut commands: Commands, q_targets: Query<Entity, Or<(Added<Player>, (Added<Ship>, Without<Pirate>))>>) {
    for ent in &q_targets {
        commands.entity(ent).insert(PirateTarget);
    }
}

fn add_pirate_ai(mut commands: Commands, q_needs_ai: Query<Entity, (With<Pirate>, Without<PirateAi>)>) {
    for ent in &q_needs_ai {
        let pilot_ent = commands.spawn((PiratePilot, DespawnWithStructure, Pilot { entity: ent })).id();
        commands
            .entity(ent)
            .insert((AiControlled, PirateAi::default(), SpeedNeedsMeasured, Pilot { entity: pilot_ent }))
            .add_child(pilot_ent);
    }
}

#[derive(Component)]
struct SpeedBeingMeasured {
    start_time: f64,
    starting_vel: Vec3,
}

#[derive(Component)]
struct SpeedNeedsMeasured;

/// This will gauge the average acceleration per second a pirate has.
///
/// Note that this function assumes optimal conditions - the pirate has an unobstructed path and there is nothing
/// that would change how fast they can move in a second.
fn measure_acceleration_per_second(
    mut commands: Commands,
    mut q_added_ai: Query<(Entity, &Velocity, &mut ShipMovement, &mut PirateAi, Option<&SpeedBeingMeasured>), With<SpeedNeedsMeasured>>,
    time: Res<Time>,
) {
    for (entity, ship_vel, mut ship_movement, mut pirate_ai, speed_being_measured) in q_added_ai.iter_mut() {
        if pirate_ai.accel_per_sec.is_none() {
            if let Some(speed_being_measured) = speed_being_measured {
                let delta = (time.elapsed_seconds_f64() - speed_being_measured.start_time) as f32;
                if delta >= 1.0 {
                    // a = v/t
                    pirate_ai.accel_per_sec = Some(speed_being_measured.starting_vel.distance(ship_vel.linvel) / delta);

                    println!("Measured acceleration per sec {:?}!", pirate_ai.accel_per_sec);

                    commands
                        .entity(entity)
                        .remove::<SpeedBeingMeasured>()
                        .remove::<SpeedNeedsMeasured>();
                }
            } else {
                commands.entity(entity).insert(SpeedBeingMeasured {
                    start_time: time.elapsed_seconds_f64(),
                    starting_vel: ship_vel.linvel,
                });
            }

            ship_movement.movement.x = 1.0;
        }
    }
}

#[derive(Component)]
struct PiratePilot;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum PirateSystemSet {
    PirateAiLogic,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(Update, PirateSystemSet::PirateAiLogic.after(LoadingSystemSet::DoneLoading))
        .add_systems(
            Update,
            (
                add_pirate_ai,
                measure_acceleration_per_second,
                add_pirate_targets,
                handle_pirate_movement,
            )
                .in_set(PirateSystemSet::PirateAiLogic)
                .chain(),
        );
}
