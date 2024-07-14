use bevy::{
    app::{App, Update},
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        query::{Or, With, Without},
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res},
    },
    hierarchy::{BuildChildren, Parent},
    math::{Quat, Vec3},
    prelude::Has,
    time::Time,
    transform::components::{GlobalTransform, Transform},
};
use bevy_rapier3d::dynamics::Velocity;
use cosmos_core::{
    ecs::NeedsDespawned,
    entities::player::Player,
    events::structure::StructureEventListenerSet,
    physics::location::Location,
    projectiles::{laser::LASER_LIVE_TIME, missile::Missile},
    structure::{
        shared::{DespawnWithStructure, MeltingDown},
        ship::{pilot::Pilot, ship_movement::ShipMovement, Ship},
        systems::{laser_cannon_system::LaserCannonSystem, StructureSystems, SystemActive},
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    persistence::{
        loading::{LoadingSystemSet, NeedsLoaded, LOADING_SCHEDULE},
        saving::{SavingSystemSet, SAVING_SCHEDULE},
        SerializedData,
    },
    structure::systems::laser_cannon_system::LASER_BASE_VELOCITY,
    universe::spawners::pirate::Pirate,
};

use super::AiControlled;

#[derive(Component)]
pub struct PirateTarget;

#[derive(Component, Default, Serialize, Deserialize)]
struct PirateAi {
    inaccuracy: f32,
    brake_check: Option<f32>,
}

impl PirateAi {
    fn randomize_inaccuracy(&mut self) {
        const INACCURACY_MULTIPLIER: f32 = 2.0;
        self.inaccuracy = (rand::random::<f32>() - 0.5) * INACCURACY_MULTIPLIER;
    }
}

const PIRATE_MAX_CHASE_DISTANCE: f32 = 20_000.0;

/// Attempt to maintain a distance of ~500 blocks from closest target
fn handle_pirate_movement(
    mut commands: Commands,
    q_laser_cannon_system: Query<Entity, With<LaserCannonSystem>>,
    mut q_pirates: Query<
        (
            Entity,
            &StructureSystems,
            &Location,
            &Velocity,
            &mut ShipMovement,
            &mut Transform,
            &mut PirateAi,
            &GlobalTransform,
        ),
        (With<Pirate>, Without<Missile>), // Without<Missile> fixes ambiguity issues
    >,
    q_parent: Query<&Parent>,
    q_velocity: Query<&Velocity>,
    q_targets: Query<(Entity, &Location, &Velocity, Has<MeltingDown>), (Without<Pirate>, With<PirateTarget>)>,
    time: Res<Time>,
) {
    for (
        pirate_ent,
        pirate_systems,
        pirate_loc,
        pirate_vel,
        mut pirate_ship_movement,
        mut pirate_transform,
        mut pirate_ai,
        pirate_g_transform,
    ) in q_pirates.iter_mut()
    {
        let Some((target_ent, target_loc, target_vel, _)) = q_targets
            .iter()
            .filter(|x| x.1.is_within_reasonable_range(pirate_loc))
            // add a large penalty for something that's melting down so they prioritize non-melting down things
            .min_by_key(|(_, this_loc, _, melting_down)| {
                // Makes it only target melting down targets if they're the only one nearby
                let melting_down_punishment = if *melting_down { 100_000_000_000_000 } else { 0 };

                this_loc.distance_sqrd(pirate_loc).floor() as u64 + melting_down_punishment
            })
        else {
            continue;
        };

        let mut target_linvel = target_vel.linvel;

        let mut entity = target_ent;
        while let Ok(parent) = q_parent.get(entity) {
            entity = parent.get();
            target_linvel += q_velocity.get(entity).map(|x| x.linvel).unwrap_or(Vec3::ZERO);
        }

        let mut pirate_linvel = pirate_vel.linvel;

        let mut entity = pirate_ent;
        while let Ok(parent) = q_parent.get(entity) {
            entity = parent.get();
            pirate_linvel += q_velocity.get(entity).map(|x| x.linvel).unwrap_or(Vec3::ZERO);
        }

        if rand::random::<f32>() < 0.01 {
            pirate_ai.randomize_inaccuracy();
        }

        let dist = target_loc.distance_sqrd(pirate_loc).sqrt();

        if dist > PIRATE_MAX_CHASE_DISTANCE {
            continue;
        }

        let laser_vel = pirate_linvel
            + Quat::from_affine3(&pirate_g_transform.affine()).mul_vec3(Vec3::new(0.0, 0.0, -LASER_BASE_VELOCITY))
            - target_linvel;

        let distance = (*target_loc - *pirate_loc).absolute_coords_f32();
        let laser_secs_to_reach_target = (distance.length() / laser_vel.length()).max(0.0);

        // Prevents a pirate from shooting the same spot repeatedly and missing and simulates inaccuracy in velocity predicting
        let max_fudge = (pirate_linvel - target_linvel).length() / 4.0;
        let velocity_fudging = pirate_ai.inaccuracy * max_fudge;

        let direction = (distance + (target_linvel - pirate_linvel + velocity_fudging) * laser_secs_to_reach_target).normalize_or_zero();

        // I don't feel like doing the angle math to make it use angular acceleration to look towards it.
        pirate_transform.look_to(direction, Vec3::Y);

        if let Some(brake_check_start) = pirate_ai.brake_check {
            pirate_ship_movement.movement = Vec3::ZERO;
            pirate_ship_movement.braking = true;
            if time.elapsed_seconds() - brake_check_start > 1.0 {
                pirate_ai.brake_check = None;
            }
        } else {
            pirate_ship_movement.braking = false;

            if dist > 500.0 {
                pirate_ship_movement.movement = Vec3::Z;
            } else {
                if pirate_vel.linvel.length() > 50.0 && rand::random::<f32>() < 0.003 {
                    pirate_ai.brake_check = Some(time.elapsed_seconds());
                }
                pirate_ship_movement.movement = -Vec3::Z;
            }
        }

        if let Ok(laser_cannon_system) = pirate_systems.query(&q_laser_cannon_system) {
            if laser_secs_to_reach_target >= LASER_LIVE_TIME.as_secs_f32() {
                commands.entity(laser_cannon_system).remove::<SystemActive>();
            } else {
                commands.entity(laser_cannon_system).insert(SystemActive);
            }
        }
    }
}

fn add_pirate_targets(
    mut commands: Commands,
    q_should_be_targets: Query<Entity, (Without<PirateTarget>, Or<(With<Player>, (With<Ship>, Without<Pirate>))>)>,
) {
    for ent in &q_should_be_targets {
        commands.entity(ent).insert(PirateTarget);
    }
}

fn add_pirate_ai(mut commands: Commands, q_needs_ai: Query<Entity, (With<Pirate>, Without<PirateAi>)>) {
    for ent in &q_needs_ai {
        let pilot_ent = commands
            .spawn((
                Name::new("Fake pirate pilot"),
                PiratePilot,
                DespawnWithStructure,
                Pilot { entity: ent },
            ))
            .id();

        let mut ai = PirateAi::default();
        ai.randomize_inaccuracy();

        commands
            .entity(ent)
            .insert((AiControlled, ai, /*SpeedNeedsMeasured,*/ Pilot { entity: pilot_ent }))
            .add_child(pilot_ent);
    }
}

fn on_melt_down(
    q_is_pirate: Query<(), With<PiratePilot>>,
    q_melting_down: Query<(Entity, &Pilot), (With<MeltingDown>, With<PirateAi>, With<AiControlled>)>,
    mut commands: Commands,
) {
    for (ent, pilot) in &q_melting_down {
        commands.entity(ent).remove::<(PirateAi, AiControlled, Pirate, Pilot)>();

        if q_is_pirate.contains(pilot.entity) {
            commands.entity(pilot.entity).insert(NeedsDespawned);
        }
    }
}

#[derive(Component)]
struct PiratePilot;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum PirateSystemSet {
    PirateAiLogic,
}

fn on_save_pirate(mut q_pirate: Query<&mut SerializedData, With<Pirate>>) {
    for mut serialized_data in q_pirate.iter_mut() {
        serialized_data.serialize_data("cosmos:pirate", &true);
    }
}

fn on_load_pirate(mut commands: Commands, query: Query<(Entity, &SerializedData), With<NeedsLoaded>>) {
    for (entity, serialized_data) in query.iter() {
        if serialized_data.deserialize_data::<bool>("cosmos:pirate").unwrap_or(false) {
            commands.entity(entity).insert(Pirate);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        PirateSystemSet::PirateAiLogic
            .after(LoadingSystemSet::DoneLoading)
            .after(StructureEventListenerSet::ChangePilotListener),
    )
    .add_systems(
        Update,
        (on_melt_down, add_pirate_ai, add_pirate_targets, handle_pirate_movement)
            .in_set(PirateSystemSet::PirateAiLogic)
            .chain(),
    )
    .add_systems(LOADING_SCHEDULE, on_load_pirate.in_set(LoadingSystemSet::DoLoading))
    .add_systems(SAVING_SCHEDULE, on_save_pirate.in_set(SavingSystemSet::DoSaving));
}
