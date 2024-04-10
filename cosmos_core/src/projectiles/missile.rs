//! A missile is something that flies in a straight line & may collide with a block, causing
//! it to take damage. Use `Missile::spawn` to create a missile.

use std::time::Duration;

use bevy::{
    core::Name,
    ecs::{
        event::EventReader,
        schedule::{IntoSystemConfigs, SystemSet},
    },
    hierarchy::Parent,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::{App, Commands, Component, Entity, Event, Query, Res, Transform, Update, Vec3},
    reflect::Reflect,
    time::Time,
    transform::components::GlobalTransform,
};
use bevy_rapier3d::{
    geometry::{ActiveEvents, ActiveHooks, Collider},
    pipeline::CollisionEvent,
    prelude::{PhysicsWorld, RigidBody, Velocity, WorldId},
};

use crate::{
    ecs::bundles::CosmosPbrBundle,
    physics::{
        collision_handling::{CollisionBlacklist, CollisionBlacklistedEntity},
        location::Location,
    },
};

/// How long a missile will stay alive for before despawning
pub const MISSILE_LIVE_TIME: Duration = Duration::from_secs(20);

#[derive(Debug, Event)]
/// The entity hit represents the entity hit by the missile
///
/// The world location the exact position the world this collision happened
///
/// The relative location is based off the hit entity's world view
/// - The relative location is how that object would perceive the point based on how it views the world
/// - This means that the relative counts the hit entity's rotation
/// - To get the world point (assuming this entity hasn't moved), simply do
///     - (That entity's rotation quaternion * relative_location) + that entity's global transform position.
pub struct MissileCollideEvent {
    entity_hit: Entity,
    local_position_hit: Vec3,
    missile_strength: f32,
}

impl MissileCollideEvent {
    /// Gets the entity this missile hit
    ///
    /// *NOTE*: Make sure to verify this entity still exists before processing it
    pub fn entity_hit(&self) -> Entity {
        self.entity_hit
    }

    /// The explosive strength of this missile
    pub fn missile_strength(&self) -> f32 {
        self.missile_strength
    }

    /// The location this missile hit relative to the entity it hit's transform.
    pub fn local_position_hit(&self) -> Vec3 {
        self.local_position_hit
    }
}

#[derive(Component)]
struct FireTime {
    time: f32,
}

#[derive(Component)]
/// A missile is something that flies in a straight line & may collide with a block, causing
/// it to take damage. Use `Missile::spawn` to create a missile.
pub struct Missile {
    /// The strength of this missile, used to calculate block damage
    pub strength: f32,
}

impl Missile {
    /// Spawns a missile with the given position & velocity
    ///
    /// * `missile_velocity` - The missile's velocity. Do not add the parent's velocity for this, use `firer_velocity` instead.
    /// * `firer_velocity` - The missile's parent's velocity.
    /// * `pbr` - This takes a PBR that contains mesh data. The location & rotation fields will be overwritten
    pub fn spawn_custom_pbr(
        location: Location,
        missile_velocity: Vec3,
        firer_velocity: Vec3,
        strength: f32,
        no_collide_entity: Option<Entity>,
        mut pbr: CosmosPbrBundle,
        time: &Time,
        world_id: WorldId,
        commands: &mut Commands,
    ) -> Entity {
        pbr.rotation = Transform::from_xyz(0.0, 0.0, 0.0)
            .looking_at(missile_velocity, Vec3::Y)
            .rotation
            .into();
        pbr.location = location;

        let mut ent_cmds = commands.spawn_empty();

        let missile_entity = ent_cmds.id();

        ent_cmds.insert((
            Name::new("Missile"),
            Missile { strength },
            pbr,
            RigidBody::Dynamic,
            Collider::cuboid(0.15, 0.15, 0.5),
            Velocity {
                linvel: missile_velocity + firer_velocity,
                ..Default::default()
            },
            FireTime {
                time: time.elapsed_seconds(),
            },
            PhysicsWorld { world_id },
            NotShadowCaster,
            ActiveEvents::COLLISION_EVENTS,
            ActiveHooks::FILTER_CONTACT_PAIRS,
            NotShadowReceiver,
        ));

        if let Some(ent) = no_collide_entity {
            ent_cmds.insert(CollisionBlacklist::single(CollisionBlacklistedEntity {
                entity: ent,
                search_parents: true,
            }));
        }

        missile_entity
    }

    /// Spawns a missile with the given position & velocity
    ///
    /// * `missile_velocity` - The missile's velocity. Do not add the parent's velocity for this, use `firer_velocity` instead.
    /// * `firer_velocity` - The missile's parent's velocity.
    pub fn spawn(
        position: Location,
        missile_velocity: Vec3,
        firer_velocity: Vec3,
        strength: f32,
        no_collide_entity: Option<Entity>,
        time: &Time,
        world_id: WorldId,
        commands: &mut Commands,
    ) -> Entity {
        Self::spawn_custom_pbr(
            position,
            missile_velocity,
            firer_velocity,
            strength,
            no_collide_entity,
            CosmosPbrBundle { ..Default::default() },
            time,
            world_id,
            commands,
        )
    }
}

#[derive(Component, Reflect)]
/// Something that will cause damage to nearby entities that it hits.
pub struct Explosion {
    /// The power of the explosion is used to calculate its radius & effectiveness against blocks.
    ///
    /// The radius of an explosion (assuming no blocks to dampen its power) is calculated as `sqrt(power)`.
    pub power: f32,
}

fn respond_to_collisions(
    mut ev_reader: EventReader<CollisionEvent>,
    q_missile: Query<(&GlobalTransform, &Missile, &CollisionBlacklist)>,
    q_parent: Query<&Parent>,
    mut commands: Commands,
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

        let Some(((g_t, missile, collision_blacklist), missile_entity, hit_entity)) = entities else {
            continue;
        };

        if !collision_blacklist.check_should_collide(hit_entity, &q_parent) {
            continue;
        }

        println!("Missile @ {} hit something! {hit_entity:?}", g_t.translation());

        commands
            .entity(missile_entity)
            .remove::<(Missile, FireTime, Collider, ActiveHooks, ActiveEvents)>()
            .insert(Explosion { power: missile.strength });
    }
}

fn despawn_missiles(mut commands: Commands, query: Query<(Entity, &FireTime, &Missile)>, time: Res<Time>) {
    for (ent, fire_time, missile) in query.iter() {
        if time.elapsed_seconds() - fire_time.time > MISSILE_LIVE_TIME.as_secs_f32() {
            println!("Missile exploded of old age");

            commands
                .entity(ent)
                .remove::<(Missile, FireTime, Collider, ActiveHooks, ActiveEvents)>()
                .insert(Explosion { power: missile.strength });
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System used for dealing with explosions that happen in the world.
///
/// Put anything that creates an explosion before [`ExplosionSystemSet::ProcessExplosions`].
pub enum ExplosionSystemSet {
    /// Put anything that creates an explosion before [`ExplosionSystemSet::ProcessExplosions`].
    ///
    /// In this set, explosions will cause damage to things they are near
    ProcessExplosions,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(Update, ExplosionSystemSet::ProcessExplosions);

    app.add_systems(
        Update,
        (respond_to_collisions, despawn_missiles)
            .before(ExplosionSystemSet::ProcessExplosions)
            .chain(),
    )
    .add_event::<MissileCollideEvent>()
    .register_type::<Explosion>();
}
