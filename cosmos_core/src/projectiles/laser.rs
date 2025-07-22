//! A laser is something that flies in a straight line & may collide with a block, causing
//! it to take damage. Use `Laser::spawn` to create a laser.

use std::time::Duration;

use bevy::{
    log::warn,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    time::Time,
};
use bevy_rapier3d::{
    plugin::{RapierContextEntityLink, WriteRapierContext},
    prelude::{LockedAxes, QueryFilter, RigidBody, Velocity},
};

use crate::{
    ecs::{NeedsDespawned, sets::FixedUpdateSet},
    netty::NoSendEntity,
    persistence::LoadingDistance,
    physics::{
        location::{Location, SetPosition},
        player_world::PlayerWorld,
    },
    structure::chunk::ChunkEntity,
};

use super::causer::Causer;

/// How long a laser will stay alive for before despawning
pub const LASER_LIVE_TIME: Duration = Duration::from_secs(5);

#[derive(Debug, Event)]
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
    causer: Option<Causer>,
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

    /// Returns the entity that caused this laser to be fired
    pub fn causer(&self) -> Option<Causer> {
        self.causer
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
    pub fn spawn<'a>(
        location: Location,
        laser_velocity: Vec3,
        firer_velocity: Vec3,
        strength: f32,
        no_collide_entity: Option<Entity>,
        time: &Time,
        context_entity_link: RapierContextEntityLink,
        commands: &'a mut Commands,
        causer: Option<Causer>,
    ) -> EntityCommands<'a> {
        let rot = Transform::from_xyz(0.0, 0.0, 0.0).looking_at(laser_velocity, Vec3::Y).rotation;

        let mut ent_cmds = commands.spawn_empty();

        ent_cmds.insert((
            Laser {
                strength,
                active: true,
                last_position: location,
            },
            location,
            Transform::from_rotation(rot),
            SetPosition::Transform,
            RigidBody::Dynamic,
            LockedAxes::ROTATION_LOCKED,
            Velocity {
                linvel: laser_velocity + firer_velocity,
                ..Default::default()
            },
            FireTime { time: time.elapsed_secs() },
            context_entity_link,
            NotShadowCaster,
            NotShadowReceiver,
            NoSendEntity,
            LoadingDistance::new(1, 1),
        ));

        if let Some(causer) = causer {
            ent_cmds.insert(causer);
        }

        if let Some(ent) = no_collide_entity {
            ent_cmds.insert(NoCollide(ent));
        }

        ent_cmds
    }
}

fn send_laser_hit_events(
    mut query: Query<
        (
            &RapierContextEntityLink,
            &Location,
            Entity,
            Option<&NoCollide>,
            &mut Laser,
            &Velocity,
            Option<&Causer>,
        ),
        With<Laser>,
    >,
    mut commands: Commands,
    mut event_writer: EventWriter<LaserCollideEvent>,
    parent_query: Query<&ChildOf>,
    chunk_parent_query: Query<&ChildOf, With<ChunkEntity>>,
    transform_query: Query<&GlobalTransform, Without<Laser>>,
    worlds: Query<&Location, With<PlayerWorld>>,
    q_rapier_context: WriteRapierContext,
) {
    for (world, location, laser_entity, no_collide_entity, mut laser, velocity, causer) in query.iter_mut() {
        if laser.active {
            let last_pos = laser.last_position;
            let delta_position = last_pos.relative_coords_to(location);
            laser.last_position = *location;

            let coords: Option<Vec3> = if let Ok(loc) = worlds.get(world.0) {
                Some(loc.relative_coords_to(location))
            } else {
                warn!("Laser playerworld not found!");
                None
            };

            let Some(coords) = coords else {
                continue;
            };

            let ray_start = coords - delta_position;

            // * 2.0 to account for checking behind the laser
            let ray_distance = ((delta_position * 2.0).dot(delta_position * 2.0)).sqrt();

            // The transform's rotation may not accurately represent the direction the laser is travelling,
            // so rather use its actual delta position for direction of travel calculations
            let ray_direction = delta_position.normalize_or_zero();

            if let Some((entity, toi)) = q_rapier_context.get(*world).cast_ray(
                ray_start, // sometimes lasers pass through things that are next to where they are spawned, thus we check starting a bit behind them
                ray_direction,
                ray_distance,
                false,
                QueryFilter::predicate(QueryFilter::default(), &|entity| {
                    if let Some(no_collide_entity) = no_collide_entity {
                        if no_collide_entity.0 == entity {
                            false
                        } else if let Ok(parent) = parent_query.get(entity) {
                            parent.parent() != no_collide_entity.0
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                }),
            ) {
                let pos = ray_start + (toi * ray_direction) + (velocity.linvel.normalize() * 0.01);

                if let Ok(parent) = chunk_parent_query.get(entity) {
                    if let Ok(transform) = transform_query.get(parent.parent()) {
                        let lph = Quat::from_affine3(&transform.affine())
                            .inverse()
                            .mul_vec3(pos - transform.translation());

                        event_writer.write(LaserCollideEvent {
                            entity_hit: parent.parent(),
                            local_position_hit: lph,
                            laser_strength: laser.strength,
                            causer: causer.copied(),
                        });
                    }
                } else if let Ok(transform) = transform_query.get(entity) {
                    let lph = Quat::from_affine3(&transform.affine())
                        .inverse()
                        .mul_vec3(pos - transform.translation());

                    event_writer.write(LaserCollideEvent {
                        entity_hit: entity,
                        local_position_hit: lph,
                        laser_strength: laser.strength,
                        causer: causer.copied(),
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
        if time.elapsed_secs() - fire_time.time > LASER_LIVE_TIME.as_secs_f32() {
            commands.entity(ent).insert(NeedsDespawned);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Laser systems should be put here
pub enum LaserSystemSet {
    /// When a laser hits something, the set that sends the [`LaserCollideEvent`] event will be here
    SendHitEvents,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(FixedUpdate, LaserSystemSet::SendHitEvents)
        .add_systems(
            FixedUpdate,
            (send_laser_hit_events.in_set(LaserSystemSet::SendHitEvents), despawn_lasers)
                .in_set(FixedUpdateSet::Main)
                .chain(),
        )
        .add_event::<LaserCollideEvent>();
}
