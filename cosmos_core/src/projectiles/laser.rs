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
    block::Block,
    ecs::{NeedsDespawned, compute_totally_accurate_global_transform, sets::FixedUpdateSet},
    events::block_events::BlockChangedEvent,
    netty::{NoSendEntity, server_laser_cannon_system_messages::LaserLoc},
    persistence::LoadingDistance,
    physics::location::{Location, SetPosition, systems::PreviousLocation},
    prelude::{BlockCoordinate, Structure, StructureBlock},
    registry::Registry,
    structure::chunk::ChunkEntity,
};

use super::causer::Causer;

/// How long a laser will stay alive for before despawning
pub const LASER_LIVE_TIME: Duration = Duration::from_secs(50);

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
    block_hit: Option<StructureBlock>,
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

    pub fn block_hit(&self) -> Option<StructureBlock> {
        self.block_hit
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
}

impl Laser {
    /// Spawns a laser with the given position & velocity
    ///
    /// * `laser_velocity` - The laser's velocity. Do not add the parent's velocity for this, use `firer_velocity` instead.
    /// * `firer_velocity` - The laser's parent's velocity.
    pub fn spawn<'a>(
        location: LaserLoc,
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

        let mut ecmds = commands.spawn_empty();

        match location {
            LaserLoc::Absolute(l) => ecmds.insert(l),
            LaserLoc::Relative { entity, offset } => ecmds.insert(SetPosition::RelativeTo { entity, offset }),
        };

        ecmds.insert((
            Laser { strength, active: true },
            Name::new("Laser"),
            Transform::from_rotation(rot),
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
            ecmds.insert(causer);
        }

        if let Some(ent) = no_collide_entity {
            ecmds.insert(NoCollide(ent));
        }

        ecmds
    }
}

fn send_laser_hit_events(
    mut query: Query<(
        &RapierContextEntityLink,
        &Location,
        Entity,
        Option<&NoCollide>,
        &mut Laser,
        &Velocity,
        Option<&Causer>,
        Option<&PreviousLocation>,
    )>,
    mut commands: Commands,
    mut event_writer: EventWriter<LaserCollideEvent>,
    parent_query: Query<&ChildOf>,
    chunk_parent_query: Query<&ChildOf, With<ChunkEntity>>,
    q_rapier_context: WriteRapierContext,
    q_transform: Query<(&Transform, Option<&ChildOf>)>,
    mut q_structure: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut evw_block_change: EventWriter<BlockChangedEvent>,
) {
    for (world, location, laser_entity, no_collide_entity, mut laser, velocity, causer, prev_loc) in query.iter_mut() {
        let Some(prev_loc) = prev_loc else {
            continue;
        };

        if laser.active {
            let last_pos = prev_loc.0;
            let delta_position = last_pos.relative_coords_to(location);

            let Some(laser_g_trans) = compute_totally_accurate_global_transform(laser_entity, &q_transform) else {
                continue;
            };

            // let coords: Option<Vec3> = if let Ok(loc) = worlds.get(world.0) {
            //     Some(loc.relative_coords_to(location))
            // } else {
            //     warn!("Laser playerworld not found!");
            //     None
            // };

            let ray_start = laser_g_trans.translation() - delta_position;

            // * 2.0 to account for checking behind the laser
            let ray_distance = ((delta_position * 2.0).dot(delta_position * 2.0)).sqrt();

            // The transform's rotation may not accurately represent the direction the laser is travelling,
            // so rather use its actual delta position for direction of travel calculations
            let ray_direction = delta_position.normalize_or_zero();

            let context = q_rapier_context.get(*world);

            if let Some((entity, toi)) = context.cast_ray(
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

                info!("Hit: {pos}");
                info!("Ray start: {ray_start:?}");
                info!("Delta pos: {delta_position:?}");

                if let Ok((mut structure, entity_hit)) = chunk_parent_query
                    .get(entity)
                    .and_then(|e| q_structure.get_mut(e.parent()).map(|s| (s, e.parent())))
                {
                    if let Some(transform) = compute_totally_accurate_global_transform(entity_hit, &q_transform) {
                        let lph = Quat::from_affine3(&transform.affine())
                            .inverse()
                            .mul_vec3(pos - transform.translation());

                        info!("Ray start: {ray_start:?}; Hit loc: {:?}", transform.translation());
                        info!("Local position hit: {lph}");

                        let block_hit = BlockCoordinate::try_from(structure.relative_coords_to_local_coords(lph.x, lph.y, lph.z))
                            .ok()
                            .filter(|c| structure.is_within_blocks(*c))
                            .map(|c| StructureBlock::new(c, entity_hit));

                        if let Some(block_hit) = block_hit {
                            info!("HIT: {:?}", block_hit.coords());

                            // structure.set_block_at(
                            //     block_hit.coords(),
                            //     blocks.from_id("cosmos:grass").unwrap(),
                            //     default(),
                            //     &blocks,
                            //     Some(&mut evw_block_change),
                            // );
                        }

                        event_writer.write(LaserCollideEvent {
                            entity_hit,
                            local_position_hit: lph,
                            block_hit,
                            laser_strength: laser.strength,
                            causer: causer.copied(),
                        });
                    }
                } else if let Some(transform) = compute_totally_accurate_global_transform(entity, &q_transform) {
                    let lph = Quat::from_affine3(&transform.affine())
                        .inverse()
                        .mul_vec3(pos - transform.translation());

                    event_writer.write(LaserCollideEvent {
                        entity_hit: entity,
                        local_position_hit: lph,
                        laser_strength: laser.strength,
                        block_hit: None,
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
                .in_set(FixedUpdateSet::PostLocationSyncingPostPhysics)
                .chain(),
        )
        .add_event::<LaserCollideEvent>();
}
