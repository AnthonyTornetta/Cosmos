//! A missile is something that flies in a straight line & may collide with a block, causing
//! it to take damage. Use `Missile::spawn` to create a missile.

use std::time::Duration;

use bevy::{
    core::Name,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::{
        App, Commands, Component, Entity, Event, EventWriter, GlobalTransform, Parent, Quat, Query, Res, Transform, Update, Vec3, With,
        Without,
    },
    time::Time,
};
use bevy_rapier3d::{
    geometry::{ActiveEvents, Sensor},
    prelude::{LockedAxes, PhysicsWorld, RapierContext, RigidBody, Velocity, WorldId, DEFAULT_WORLD_ID},
};

use crate::{
    ecs::{bundles::CosmosPbrBundle, NeedsDespawned},
    netty::NoSendEntity,
    physics::{
        location::Location,
        player_world::{PlayerWorld, WorldWithin},
    },
};

/// How long a missile will stay alive for before despawning
pub const MISSILE_LIVE_TIME: Duration = Duration::from_secs(5);

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
/// This is used to prevent the missile from colliding with the entity that fired it
/// If this component is found on the object that it was fired on, then no collision will be registered
pub struct NoCollide(Entity);

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
            LockedAxes::ROTATION_LOCKED,
            Velocity {
                linvel: missile_velocity + firer_velocity,
                ..Default::default()
            },
            FireTime {
                time: time.elapsed_seconds(),
            },
            Sensor,
            PhysicsWorld { world_id },
            NotShadowCaster,
            ActiveEvents::COLLISION_EVENTS,
            NotShadowReceiver,
        ));

        if let Some(ent) = no_collide_entity {
            ent_cmds.insert(NoCollide(ent));
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

fn handle_events(
    mut query: Query<
        (
            Option<&PhysicsWorld>,
            &Location,
            Entity,
            Option<&NoCollide>,
            &mut Missile,
            &Velocity,
            Option<&WorldWithin>,
        ),
        With<Missile>,
    >,
    mut commands: Commands,
    mut event_writer: EventWriter<MissileCollideEvent>,
    rapier_context: Res<RapierContext>,
    parent_query: Query<&Parent>,
    transform_query: Query<&GlobalTransform, Without<Missile>>,
    worlds: Query<(&Location, &PhysicsWorld, Entity), With<PlayerWorld>>,
) {
    for (world, location, missile_entity, no_collide_entity, mut missile, velocity, world_within) in query.iter_mut() {
        // if missile.active {
        //     let last_pos = missile.last_position;
        //     let delta_position = last_pos.relative_coords_to(location);
        //     missile.last_position = *location;

        //     let world_id = world.map(|bw| bw.world_id).unwrap_or(DEFAULT_WORLD_ID);

        //     let coords: Option<Vec3> = world_within
        //         .map(|world_within| {
        //             if let Ok((loc, _, _)) = worlds.get(world_within.0) {
        //                 Some(loc.relative_coords_to(location))
        //             } else {
        //                 warn!("Missile playerworld not found!");
        //                 None
        //             }
        //         })
        //         .unwrap_or(None);

        //     let Some(coords) = coords else {
        //         continue;
        //     };

        //     let ray_start = coords - delta_position;

        //     // * 2.0 to account for checking behind the missile
        //     let ray_distance = ((delta_position * 2.0).dot(delta_position * 2.0)).sqrt();

        //     // The transform's rotation may not accurately represent the direction the missile is travelling,
        //     // so rather use its actual delta position for direction of travel calculations
        //     let ray_direction = delta_position.normalize_or_zero();

        //     if let Ok(Some((entity, toi))) = rapier_context.cast_ray(
        //         world_id,
        //         ray_start, // sometimes missiles pass through things that are next to where they are spawned, thus we check starting a bit behind them
        //         ray_direction,
        //         ray_distance,
        //         false,
        //         QueryFilter::predicate(QueryFilter::default(), &|entity| {
        //             if let Some(no_collide_entity) = no_collide_entity {
        //                 if no_collide_entity.0 == entity {
        //                     false
        //                 } else if let Ok(parent) = parent_query.get(entity) {
        //                     parent.get() != no_collide_entity.0
        //                 } else {
        //                     false
        //                 }
        //             } else {
        //                 true
        //             }
        //         }),
        //     ) {
        //         let pos = ray_start + (toi * ray_direction) + (velocity.linvel.normalize() * 0.01);

        //         if let Ok(parent) = parent_query.get(entity) {
        //             if let Ok(transform) = transform_query.get(parent.get()) {
        //                 let lph = Quat::from_affine3(&transform.affine())
        //                     .inverse()
        //                     .mul_vec3(pos - transform.translation());

        //                 event_writer.send(MissileCollideEvent {
        //                     entity_hit: entity,
        //                     local_position_hit: lph,
        //                     missile_strength: missile.strength,
        //                 });
        //             }
        //         } else if let Ok(transform) = transform_query.get(entity) {
        //             let lph = Quat::from_affine3(&transform.affine())
        //                 .inverse()
        //                 .mul_vec3(pos - transform.translation());

        //             event_writer.send(MissileCollideEvent {
        //                 entity_hit: entity,
        //                 local_position_hit: lph,
        //                 missile_strength: missile.strength,
        //             });
        //         }

        //         missile.active = false;
        //         commands.entity(missile_entity).insert(NeedsDespawned);
        //     }
        // }
    }
}

fn despawn_missiles(mut commands: Commands, query: Query<(Entity, &FireTime), With<Missile>>, time: Res<Time>) {
    for (ent, fire_time) in query.iter() {
        if time.elapsed_seconds() - fire_time.time > MISSILE_LIVE_TIME.as_secs_f32() {
            commands.entity(ent).insert(NeedsDespawned);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (handle_events, despawn_missiles))
        .add_event::<MissileCollideEvent>();
}
