//! A laser is something that flies in a straight line & may collide with a block, causing
//! it to take damage. Use `Laser::spawn` to create a laser.

use bevy::{
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::{
        warn, App, Commands, Component, Entity, EventWriter, GlobalTransform, Parent, Quat, Query, Res, Transform, Vec3, With, Without,
    },
    time::Time,
};
use bevy_rapier3d::prelude::{
    ActiveEvents, ActiveHooks, LockedAxes, PhysicsWorld, QueryFilter, RapierContext, RigidBody, Sensor, Velocity, WorldId, DEFAULT_WORLD_ID,
};

use crate::{
    ecs::{bundles::CosmosPbrBundle, NeedsDespawned},
    netty::NoSendEntity,
    physics::{
        location::Location,
        player_world::{PlayerWorld, WorldWithin},
    },
};

#[derive(Debug)]
/// The entity hit represents the entity hit by the laser
///
/// The world location the exact position the world this collision happened
///
/// The relative location is based off the hit entity's world view
/// - The relative location is how that object would perceive the point based on how it views the world
/// - This means that the relative counts the hit entity's rotation
/// - To get the world point (assuming this entity hasn't moved), simply do
///     - (That entity's rotation quaternion * relative_location) + that entity's global transform position.
pub struct LaserCollideEvent {
    entity_hit: Entity,
    local_position_hit: Vec3,
    laser_strength: f32,
}

impl LaserCollideEvent {
    /// Gets the entity this laser hit
    ///
    /// *NOTE*: Make sure to verify this entity still exists before processing it
    pub fn entity_hit(&self) -> Entity {
        self.entity_hit
    }

    /// The strength of this laser
    pub fn laser_strength(&self) -> f32 {
        self.laser_strength
    }

    /// The location this laser hit relative to the entity it hit's transform.
    pub fn local_position_hit(&self) -> Vec3 {
        self.local_position_hit
    }
}

#[derive(Component)]
/// This is used to prevent the laser from colliding with the entity that fired it
/// If this component is found on the object that it was fired on, then no collision will be registered
pub struct NoCollide(Entity);

#[derive(Component)]
struct FireTime {
    time: f32,
}

#[derive(Component)]
/// A laser is something that flies in a straight line & may collide with a block, causing
/// it to take damage. Use `Laser::spawn` to create a laser.
pub struct Laser {
    /// commands despawning entity isn't instant, but changing this field is.
    /// Thus, this field should always be checked when determining if a laser should break/damage something.
    active: bool,

    /// The strength of this laser, used to calculate block damage
    pub strength: f32,

    /// Used to calculate an extra-big hitbox to account for hitting something right when it fires
    last_position: Location,
}

impl Laser {
    /// Spawns a laser with the given position & velocity
    ///
    /// * `laser_velocity` - The laser's velocity. Do not add the parent's velocity for this, use `firer_velocity` instead.
    /// * `firer_velocity` - The laser's parent's velocity.
    /// * `pbr` - This takes a PBR that contains mesh data. The location & rotation fields will be overwritten
    pub fn spawn_custom_pbr(
        location: Location,
        laser_velocity: Vec3,
        firer_velocity: Vec3,
        strength: f32,
        no_collide_entity: Option<Entity>,
        mut pbr: CosmosPbrBundle,
        time: &Time,
        world_id: WorldId,
        commands: &mut Commands,
    ) -> Entity {
        pbr.rotation = Transform::from_xyz(0.0, 0.0, 0.0)
            .looking_at(laser_velocity, Vec3::Y)
            .rotation
            .into();
        pbr.location = location;

        let mut ent_cmds = commands.spawn_empty();

        let laser_entity = ent_cmds.id();

        ent_cmds.insert((
            Laser {
                strength,
                active: true,
                last_position: location,
            },
            pbr,
            RigidBody::Dynamic,
            LockedAxes::ROTATION_LOCKED,
            Velocity {
                linvel: laser_velocity + firer_velocity,
                ..Default::default()
            },
            FireTime {
                time: time.elapsed_seconds(),
            },
            PhysicsWorld { world_id },
            ActiveEvents::COLLISION_EVENTS,
            ActiveHooks::MODIFY_SOLVER_CONTACTS,
            Sensor,
            NotShadowCaster,
            NotShadowReceiver,
            NoSendEntity,
        ));

        if let Some(ent) = no_collide_entity {
            ent_cmds.insert(NoCollide(ent));
        }

        laser_entity
    }

    /// Spawns a laser with the given position & velocity
    ///
    /// * `laser_velocity` - The laser's velocity. Do not add the parent's velocity for this, use `firer_velocity` instead.
    /// * `firer_velocity` - The laser's parent's velocity.
    pub fn spawn(
        position: Location,
        laser_velocity: Vec3,
        firer_velocity: Vec3,
        strength: f32,
        no_collide_entity: Option<Entity>,
        time: &Time,
        world_id: WorldId,
        commands: &mut Commands,
    ) -> Entity {
        Self::spawn_custom_pbr(
            position,
            laser_velocity,
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
            &mut Laser,
            &Velocity,
            Option<&WorldWithin>,
        ),
        With<Laser>,
    >,
    mut commands: Commands,
    mut event_writer: EventWriter<LaserCollideEvent>,
    rapier_context: Res<RapierContext>,
    parent_query: Query<&Parent>,
    transform_query: Query<&GlobalTransform, Without<Laser>>,
    worlds: Query<(&Location, &PhysicsWorld, Entity), With<PlayerWorld>>,
) {
    for (world, location, laser_entity, no_collide_entity, mut laser, velocity, world_within) in query.iter_mut() {
        if laser.active {
            let last_pos = laser.last_position;
            let delta_position = last_pos.relative_coords_to(location);
            laser.last_position = *location;

            let world_id = world.map(|bw| bw.world_id).unwrap_or(DEFAULT_WORLD_ID);

            let coords: Option<Vec3> = world_within
                .map(|world_within| {
                    if let Ok((loc, _, _)) = worlds.get(world_within.0) {
                        Some(loc.relative_coords_to(location))
                    } else {
                        warn!("Laser playerworld not found!");
                        None
                    }
                })
                .unwrap_or(None);

            let Some(coords) = coords else {
                continue;
            };

            let ray_start = coords - delta_position;

            // * 2.0 to account for checking behind the laser
            let ray_distance = ((delta_position * 2.0).dot(delta_position * 2.0)).sqrt();

            // The transform's rotation may not accurately represent the direction the laser is travelling,
            // so rather use its actual delta position for direction of travel calculations
            let ray_direction = delta_position.normalize_or_zero();

            if let Ok(Some((entity, toi))) = rapier_context.cast_ray(
                world_id,
                ray_start, // sometimes lasers pass through things that are next to where they are spawned, thus we check starting a bit behind them
                ray_direction,
                ray_distance,
                false,
                QueryFilter::predicate(QueryFilter::default(), &|entity| {
                    if let Some(no_collide_entity) = no_collide_entity {
                        if no_collide_entity.0 == entity {
                            false
                        } else if let Ok(parent) = parent_query.get(entity) {
                            parent.get() != no_collide_entity.0
                        } else {
                            false
                        }
                    } else {
                        true
                    }
                }),
            ) {
                let pos = ray_start + (toi * ray_direction) + (velocity.linvel.normalize() * 0.01);

                if let Ok(parent) = parent_query.get(entity) {
                    if let Ok(transform) = transform_query.get(parent.get()) {
                        let lph = Quat::from_affine3(&transform.affine())
                            .inverse()
                            .mul_vec3(pos - transform.translation());

                        event_writer.send(LaserCollideEvent {
                            entity_hit: entity,
                            local_position_hit: lph,
                            laser_strength: laser.strength,
                        });
                    }
                } else if let Ok(transform) = transform_query.get(entity) {
                    let lph = Quat::from_affine3(&transform.affine())
                        .inverse()
                        .mul_vec3(pos - transform.translation());

                    event_writer.send(LaserCollideEvent {
                        entity_hit: entity,
                        local_position_hit: lph,
                        laser_strength: laser.strength,
                    });
                }

                laser.active = false;
                commands.entity(laser_entity).insert(NeedsDespawned);
            }
        }
    }
}

fn despawn_lasers(mut commands: Commands, query: Query<(Entity, &FireTime), With<Laser>>, time: Res<Time>) {
    for (ent, fire_time) in query.iter() {
        if time.elapsed_seconds() - fire_time.time > 5.0 {
            commands.entity(ent).insert(NeedsDespawned);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems((handle_events, despawn_lasers)).add_event::<LaserCollideEvent>();
}
