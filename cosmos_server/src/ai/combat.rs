use bevy::prelude::*;
use bevy_rapier3d::dynamics::Velocity;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    events::structure::StructureMessageListenerSet,
    netty::sync::IdentifiableComponent,
    physics::location::Location,
    projectiles::{laser::LASER_LIVE_TIME, missile::Missile},
    state::GameState,
    structure::{
        StructureTypeSet,
        ship::ship_movement::{ShipMovement, ShipMovementSet},
        systems::{
            StructureSystems, SystemActive,
            laser_cannon_system::LaserCannonSystem,
            missile_launcher_system::{MissileLauncherFocus, MissileLauncherSystem},
        },
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    persistence::{
        loading::LoadingSystemSet,
        make_persistent::{DefaultPersistentComponent, make_persistent},
    },
    structure::systems::laser_cannon_system::LASER_BASE_VELOCITY,
};

use super::AiControlled;

#[derive(Component, Debug, Reflect)]
pub struct AiTargetting(pub Entity);

#[derive(Component, Serialize, Deserialize, Debug, Reflect)]
pub struct CombatAi {
    pub inaccuracy: f32,
    pub brake_check: Option<f32>,
    pub max_chase_distance: f32,
}

impl IdentifiableComponent for CombatAi {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:combat_ai"
    }
}

impl DefaultPersistentComponent for CombatAi {}

impl Default for CombatAi {
    fn default() -> Self {
        Self {
            inaccuracy: 0.0,
            brake_check: None,
            max_chase_distance: 20_000.0,
        }
    }
}

impl CombatAi {
    pub fn randomize_inaccuracy(&mut self) {
        const INACCURACY_MULTIPLIER: f32 = 2.0;
        self.inaccuracy = (rand::random::<f32>() - 0.5) * INACCURACY_MULTIPLIER;
    }
}

/// Attempt to maintain a distance of ~500 blocks from closest target
fn handle_combat_ai(
    mut commands: Commands,
    q_laser_cannon_system: Query<Entity, With<LaserCannonSystem>>,
    q_missile_system: Query<(Entity, &MissileLauncherFocus), With<MissileLauncherSystem>>,
    mut q_pirates: Query<
        (
            Entity,
            &StructureSystems,
            &Location,
            &Velocity,
            &mut ShipMovement,
            &mut Transform,
            &mut CombatAi,
            &AiTargetting,
            &GlobalTransform,
        ),
        (Without<Missile>, With<AiControlled>), // Without<Missile> fixes ambiguity issues
    >,
    q_parent: Query<&ChildOf>,
    q_velocity: Query<&Velocity>,
    q_targets: Query<(Entity, &Location, &Velocity)>,
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
        targetting,
        pirate_g_transform,
    ) in q_pirates.iter_mut()
    {
        let Ok((target_ent, target_loc, target_vel)) = q_targets.get(targetting.0) else {
            continue;
        };

        let mut target_linvel = target_vel.linvel;

        let mut entity = target_ent;
        while let Ok(parent) = q_parent.get(entity) {
            entity = parent.parent();
            target_linvel += q_velocity.get(entity).map(|x| x.linvel).unwrap_or(Vec3::ZERO);
        }

        let mut this_linvel = pirate_vel.linvel;

        let mut entity = pirate_ent;
        while let Ok(parent) = q_parent.get(entity) {
            entity = parent.parent();
            this_linvel += q_velocity.get(entity).map(|x| x.linvel).unwrap_or(Vec3::ZERO);
        }

        if rand::random::<f32>() < 0.01 {
            pirate_ai.randomize_inaccuracy();
        }

        let dist = target_loc.distance_sqrd(pirate_loc).sqrt();

        if dist > pirate_ai.max_chase_distance {
            pirate_ship_movement.movement = Vec3::Z;
            continue;
        }

        let laser_vel = this_linvel + Quat::from_affine3(&pirate_g_transform.affine()).mul_vec3(Vec3::new(0.0, 0.0, -LASER_BASE_VELOCITY))
            - target_linvel;

        let distance = (*target_loc - *pirate_loc).absolute_coords_f32();
        let laser_secs_to_reach_target = (distance.length() / laser_vel.length()).max(0.0);

        // Prevents a pirate from shooting the same spot repeatedly and missing and simulates inaccuracy in velocity predicting
        let max_fudge = (this_linvel - target_linvel).length() / 4.0;
        let velocity_fudging = pirate_ai.inaccuracy * max_fudge;

        let direction = (distance + (target_linvel - this_linvel + velocity_fudging) * laser_secs_to_reach_target).normalize_or_zero();

        // I don't feel like doing the angle math to make it use angular acceleration to look towards it.
        pirate_transform.look_to(direction, Vec3::Y);

        if let Some(brake_check_start) = pirate_ai.brake_check {
            pirate_ship_movement.movement = Vec3::ZERO;
            pirate_ship_movement.braking = true;
            if time.elapsed_secs() - brake_check_start > 1.0 {
                pirate_ai.brake_check = None;
            }
        } else {
            pirate_ship_movement.braking = false;

            if dist > 200.0 {
                pirate_ship_movement.movement = Vec3::Z;
            } else {
                if pirate_vel.linvel.length() > 50.0 && rand::random::<f32>() < 0.003 {
                    pirate_ai.brake_check = Some(time.elapsed_secs());
                }
                pirate_ship_movement.movement = -Vec3::Z;
            }
        }

        if let Ok(laser_cannon_system) = pirate_systems.query(&q_laser_cannon_system) {
            if laser_secs_to_reach_target >= LASER_LIVE_TIME.as_secs_f32() {
                commands.entity(laser_cannon_system).remove::<SystemActive>();
            } else {
                commands.entity(laser_cannon_system).insert(SystemActive::Primary);
            }
        }

        if let Ok((laser_cannon_system, focus)) = pirate_systems.query(&q_missile_system) {
            if focus.locked_on_to() != Some(target_ent) {
                commands.entity(laser_cannon_system).remove::<SystemActive>();
            } else {
                commands.entity(laser_cannon_system).insert(SystemActive::Primary);
            }
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum CombatAiSystemSet {
    CombatAiLogic,
}

pub(super) fn register(app: &mut App) {
    make_persistent::<CombatAi>(app);

    app.configure_sets(
        FixedUpdate,
        CombatAiSystemSet::CombatAiLogic
            .in_set(StructureTypeSet::Ship)
            .after(LoadingSystemSet::DoneLoading)
            .after(StructureMessageListenerSet::ChangePilotListener),
    )
    .register_type::<AiTargetting>()
    .register_type::<CombatAi>()
    .add_systems(
        FixedUpdate,
        (handle_combat_ai.before(ShipMovementSet::RemoveShipMovement),)
            .run_if(in_state(GameState::Playing))
            .in_set(FixedUpdateSet::Main)
            .in_set(CombatAiSystemSet::CombatAiLogic)
            .chain(),
    );
}
