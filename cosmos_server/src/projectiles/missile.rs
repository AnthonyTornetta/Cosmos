//! Server-related missile logic

use std::time::Duration;

use bevy::{
    ecs::{component::Component, event::EventReader, schedule::IntoSystemConfigs},
    hierarchy::Parent,
    log::info,
    math::Vec3,
    prelude::{App, Commands, Entity, Query, Res, Update, With},
    time::Time,
    transform::components::{GlobalTransform, Transform},
};
use bevy_rapier3d::{
    dynamics::{ExternalImpulse, Velocity},
    pipeline::CollisionEvent,
    plugin::{RapierContextEntityLink, ReadRapierContext},
    prelude::{ReadMassProperties, RigidBody},
};

use cosmos_core::{
    ecs::NeedsDespawned,
    netty::system_sets::NetworkingSystemsSet,
    persistence::LoadingDistance,
    physics::{
        collision_handling::CollisionBlacklist,
        location::{CosmosBundleSet, Location, LocationPhysicsSet},
    },
    projectiles::missile::{Explosion, ExplosionSystemSet, Missile},
    structure::StructureTypeSet,
};

#[derive(Component)]
/// Represents which entity the missile should be targetting
pub struct MissileTargetting {
    /// Makes the missile diverge from the origin a bit
    pub targetting_fudge: Vec3,
    /// The entity being targetted
    pub targetting: Entity,
}

fn look_and_move_towards_target(
    mut q_targetting_missiles: Query<(&Location, &mut Transform, &Velocity, &MissileTargetting, &ReadMassProperties)>,
    q_targets: Query<(&Location, &Velocity)>,
    time: Res<Time>,
) {
    for (missile_loc, mut missile_trans, missile_vel, missile_targetting, mass) in &mut q_targetting_missiles {
        let Ok((target_loc, target_vel)) = q_targets.get(missile_targetting.targetting) else {
            continue;
        };

        let target_loc = *target_loc + missile_targetting.targetting_fudge;

        if mass.mass == 0.0 {
            // Wait for physics engine to update mass properties
            continue;
        }

        let missile_accel = MISSILE_IMPULSE_PER_SEC / mass.mass;
        let d = (target_loc - *missile_loc).absolute_coords_f32().length();

        let a = missile_accel;
        let v = missile_vel.linvel.length();
        let t_vel = target_vel.linvel.length();

        let missile_secs_to_reach_target = (-(v - t_vel) + ((v - t_vel).powf(2.0) + 2.0 * a * d).sqrt()) / a;

        let relative_velocity = target_vel.linvel - missile_vel.linvel;

        let direction = ((target_loc - *missile_loc) + (relative_velocity * missile_secs_to_reach_target))
            .absolute_coords_f32()
            .normalize_or_zero();

        let cur_forward = missile_trans.forward();

        let dir_lerped = cur_forward.lerp(direction, time.delta_secs().min(1.0));

        missile_trans.look_to(dir_lerped, Vec3::Y);
    }
}

const MISSILE_IMPULSE_PER_SEC: f32 = 4.5;

fn apply_missile_thrust(mut commands: Commands, time: Res<Time>, q_missiles: Query<(Entity, &GlobalTransform), With<Missile>>) {
    for (ent, g_trans) in &q_missiles {
        commands.entity(ent).insert(ExternalImpulse {
            impulse: g_trans.forward() * MISSILE_IMPULSE_PER_SEC * time.delta_secs(),
            ..Default::default()
        });
    }
}

fn respond_to_collisions(
    mut ev_reader: EventReader<CollisionEvent>,
    q_missile: Query<(
        &Location,
        &Velocity,
        &Missile,
        &CollisionBlacklist,
        &GlobalTransform,
        &RapierContextEntityLink,
    )>,
    q_g_trans: Query<(&Location, &GlobalTransform)>,
    q_parent: Query<&Parent>,
    mut commands: Commands,
    context_access: ReadRapierContext,
) {
    for ev in ev_reader.read() {
        let &CollisionEvent::Started(e1, e2, _) = ev else {
            continue;
        };

        let entities = if let Ok(missile) = q_missile.get(e1) {
            Some((missile, e1, e2))
        } else if let Ok(missile) = q_missile.get(e2) {
            Some((missile, e2, e1))
        } else {
            None
        };

        let Some(((location, velocity, missile, collision_blacklist, g_trans, link), missile_entity, hit_entity)) = entities else {
            continue;
        };

        if !collision_blacklist.check_should_collide(hit_entity, &q_parent) {
            continue;
        }

        commands.entity(missile_entity).insert(NeedsDespawned);

        info!("Spawning explosion @ {}", *location,);
        commands.spawn((
            *location,
            *velocity,
            RigidBody::Dynamic,
            LoadingDistance::new(1, 2),
            Explosion {
                power: missile.strength,
                color: missile.color,
            },
        ));
    }
}

fn despawn_missiles(mut commands: Commands, mut query: Query<(Entity, &Velocity, &Location, &mut Missile)>, time: Res<Time>) {
    for (ent, velocity, location, mut missile) in query.iter_mut() {
        missile.lifetime = missile
            .lifetime
            .checked_sub(Duration::from_secs_f32(time.delta_secs()))
            .unwrap_or(Duration::ZERO);

        if missile.lifetime == Duration::ZERO {
            commands.entity(ent).insert(NeedsDespawned);

            commands.spawn((
                *location,
                *velocity,
                RigidBody::Dynamic,
                LoadingDistance::new(1, 2),
                Explosion {
                    power: missile.strength,
                    color: missile.color,
                },
            ));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (respond_to_collisions.before(NetworkingSystemsSet::SyncComponents), despawn_missiles)
            .before(ExplosionSystemSet::PreProcessExplosions)
            .after(LocationPhysicsSet::DoPhysics)
            .chain(),
    );

    app.add_systems(
        Update,
        (
            look_and_move_towards_target.ambiguous_with(StructureTypeSet::Ship),
            apply_missile_thrust,
        )
            .after(CosmosBundleSet::HandleCosmosBundles)
            .in_set(NetworkingSystemsSet::Between)
            .chain(),
    );
}
