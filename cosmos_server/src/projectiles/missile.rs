//! Server-related missile logic

use std::time::Duration;

use bevy::{
    ecs::{
        component::Component,
        event::{EventReader, EventWriter},
        query::{Added, Or, Without},
        schedule::IntoSystemConfigs,
    },
    hierarchy::Parent,
    log::info,
    math::{Quat, Vec3},
    prelude::{App, Commands, Entity, Query, Res, Update, With},
    time::Time,
    transform::components::{GlobalTransform, Transform},
    utils::HashSet,
};
use bevy_rapier3d::{
    dynamics::{ExternalImpulse, Velocity},
    geometry::{Collider, Sensor},
    pipeline::{CollisionEvent, QueryFilter},
    plugin::RapierContext,
    prelude::{PhysicsWorld, RigidBody},
};

use cosmos_core::{
    block::Block,
    ecs::NeedsDespawned,
    events::block_events::BlockChangedEvent,
    persistence::LoadingDistance,
    physics::{
        collision_handling::CollisionBlacklist,
        location::{CosmosBundleSet, Location},
        player_world::{PlayerWorld, WorldWithin},
    },
    projectiles::missile::{Explosion, ExplosionSystemSet, Missile},
    registry::Registry,
    structure::{
        chunk::ChunkEntity,
        coordinates::{BlockCoordinate, UnboundBlockCoordinate, UnboundCoordinateType},
        Structure,
    },
};

use crate::netty::sync::sync_bodies::DontNotifyClientOfDespawn;

/// 1 unit of explosion power = this amount of health. Bigger this number is, the more damage explosives will do.
const HEALTH_PER_EXPLOSION_POWER: f32 = 8.0;

fn respond_to_explosion(
    mut commands: Commands,
    q_explosions: Query<(Entity, &Location, &WorldWithin, Option<&PhysicsWorld>, &Explosion), Added<Explosion>>,
    q_player_world: Query<&Location, With<PlayerWorld>>,
    q_excluded: Query<(), Or<(With<Explosion>, Without<Collider>, With<Sensor>)>>,

    mut q_structure: Query<(&GlobalTransform, &Location, &mut Structure)>,
    context: Res<RapierContext>,

    q_chunk: Query<&ChunkEntity>,
    blocks_registry: Res<Registry<Block>>,
    mut ev_writer: EventWriter<BlockChangedEvent>,
) {
    for (ent, explosion_loc, world_within, physics_world, explosion) in q_explosions.iter() {
        commands.entity(ent).insert((NeedsDespawned, DontNotifyClientOfDespawn));

        let Ok(player_world_loc) = q_player_world.get(world_within.0) else {
            continue;
        };

        let max_radius = explosion.power.sqrt();

        let physics_world = physics_world.copied().unwrap_or_default();

        // Have to do this by hand because `GlobalTransform` doesn't have enough time to
        // propagate down
        let rapier_coordinates = (*explosion_loc - *player_world_loc).absolute_coords_f32();

        let mut hits = vec![];

        context
            .intersections_with_shape(
                physics_world.world_id,
                rapier_coordinates,
                Quat::IDENTITY,
                &Collider::ball(max_radius),
                QueryFilter::default().exclude_collider(ent).predicate(&|x| !q_excluded.contains(x)),
                |hit_entity| {
                    hits.push(hit_entity);

                    true
                },
            )
            .expect("Invalid world id used in explosion!");

        info!("Missile hit entities: {hits:?}");

        let mut ents = HashSet::new();
        for ent in hits {
            if let Ok(chunk_ent) = q_chunk.get(ent) {
                ents.insert(chunk_ent.structure_entity);
            } else {
                ents.insert(ent);
            }
        }

        let max_block_radius = max_radius.ceil() as UnboundCoordinateType;
        let max_radius_sqrd = max_radius * max_radius;

        for &hit in ents.iter() {
            let Ok((structure_g_trans, structure_loc, mut structure)) = q_structure.get_mut(hit) else {
                continue;
            };
            let explosion_relative_position =
                structure_g_trans.affine().inverse().matrix3 * (*explosion_loc - *structure_loc).absolute_coords_f32();

            let local_coords = structure.relative_coords_to_local_coords(
                explosion_relative_position.x,
                explosion_relative_position.y,
                explosion_relative_position.z,
            );

            // Intermediate Vec to please the borrow checker
            let hit_blocks = structure
                .block_iter(
                    local_coords - UnboundBlockCoordinate::splat(max_block_radius),
                    local_coords + UnboundBlockCoordinate::splat(max_block_radius),
                    true, // Include air false is broken for some reason
                )
                .map(|x| x.coords())
                .filter(|&coords| structure.has_block_at(coords)) // Remove this once `include_air` works.
                .flat_map(|this_block| {
                    calculate_block_explosion_power(
                        &structure,
                        this_block,
                        explosion_relative_position,
                        explosion,
                        &blocks_registry,
                        max_radius_sqrd,
                    )
                })
                .collect::<Vec<(BlockCoordinate, f32)>>();

            for (block, explosion_power) in hit_blocks {
                let cur_health = structure.get_block_health(block, &blocks_registry);
                structure.set_block_health(block, cur_health - explosion_power * HEALTH_PER_EXPLOSION_POWER, &blocks_registry);

                if structure.get_block_health(block, &blocks_registry) <= 0.0 {
                    structure.remove_block_at(block, &blocks_registry, Some(&mut ev_writer));
                }
            }
        }
    }
}

/// Finds how much explosive power should be applied to a block coordinate from the explosive's position.
///
/// Returns None if this block has been shielded by other blocks.
fn calculate_block_explosion_power(
    structure: &Structure,
    this_block: BlockCoordinate,
    explosion_relative_position: Vec3,
    explosion: &Explosion,
    blocks_registry: &Registry<Block>,
    max_distance_sqrd: f32,
) -> Option<(BlockCoordinate, f32)> {
    let block_pos = structure.block_relative_position(this_block);

    let distance = block_pos - explosion_relative_position;

    let mut remaining_explosion_power = explosion.power;

    for intercepting_block in structure
        .raycast_iter(explosion_relative_position, distance.normalize_or_zero(), distance.length(), false)
        .filter(|&intercepting_block| intercepting_block != this_block)
    {
        remaining_explosion_power -= structure.get_block_health(intercepting_block, blocks_registry) / HEALTH_PER_EXPLOSION_POWER;

        let block_pos = structure.block_relative_position(intercepting_block);
        // exponential decay is intended
        let decay_percent = block_pos.distance_squared(explosion_relative_position) / max_distance_sqrd;

        if remaining_explosion_power * (1.0 - decay_percent) <= 0.0 {
            return None;
        }
    }

    let block_pos = structure.block_relative_position(this_block);
    // exponential decay is intended
    let decay_percent = block_pos.distance_squared(explosion_relative_position) / max_distance_sqrd;

    remaining_explosion_power *= 1.0 - decay_percent;

    if remaining_explosion_power > 0.0 {
        Some((this_block, remaining_explosion_power))
    } else {
        None
    }
}

#[derive(Component)]
/// Represents which entity the missile should be targetting
pub struct MissileTargetting {
    /// Makes the missile diverge from the origin a bit
    pub targetting_fudge: Vec3,
    /// The entity being targetted
    pub targetting: Entity,
}

fn print_rot(q_mis: Query<&Transform, With<Missile>>) {
    for x in &q_mis {
        println!("{}", x.rotation);
    }
}

fn look_towards_target(mut q_targetting_missiles: Query<(&Location, &mut Transform, &MissileTargetting)>, q_targets: Query<&Location>) {
    for (missile_loc, mut missile_trans, missile_targetting) in &mut q_targetting_missiles {
        let Ok(target_loc) = q_targets.get(missile_targetting.targetting) else {
            continue;
        };

        let direction = (*target_loc - *missile_loc).absolute_coords_f32().normalize_or_zero() + missile_targetting.targetting_fudge;
        missile_trans.look_to(direction, Vec3::Y);
    }
}

const MISSILE_IMPULSE_PER_SEC: f32 = 1.5;

fn apply_missile_thrust(mut commands: Commands, time: Res<Time>, q_missiles: Query<(Entity, &GlobalTransform), With<Missile>>) {
    for (ent, g_trans) in &q_missiles {
        commands.entity(ent).insert(ExternalImpulse {
            impulse: g_trans.forward() * MISSILE_IMPULSE_PER_SEC * time.delta_seconds(),
            ..Default::default()
        });
    }
}

fn respond_to_collisions(
    mut ev_reader: EventReader<CollisionEvent>,
    q_missile: Query<(&Location, &Velocity, &Missile, &CollisionBlacklist)>,
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

        let Some(((location, velocity, missile, collision_blacklist), missile_entity, hit_entity)) = entities else {
            continue;
        };

        if !collision_blacklist.check_should_collide(hit_entity, &q_parent) {
            continue;
        }

        commands.entity(missile_entity).insert(NeedsDespawned);

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
            .checked_sub(Duration::from_secs_f32(time.delta_seconds()))
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
        (respond_to_collisions, despawn_missiles)
            .before(ExplosionSystemSet::PreProcessExplosions)
            .before(CosmosBundleSet::HandleCosmosBundles)
            .chain(),
    );

    app.add_systems(Update, respond_to_explosion.in_set(ExplosionSystemSet::ProcessExplosions))
        .add_systems(
            Update,
            (print_rot, look_towards_target, apply_missile_thrust)
                .after(CosmosBundleSet::HandleCosmosBundles)
                .chain(),
        );
}
