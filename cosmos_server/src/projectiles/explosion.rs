//! Server-related logic for explosions

use bevy::{platform::collections::HashSet, prelude::*};
use bevy_rapier3d::{
    geometry::Collider,
    pipeline::QueryFilter,
    plugin::{RapierContextEntityLink, ReadRapierContext},
};

use cosmos_core::{
    block::{Block, block_events::BlockMessagesSet},
    ecs::{NeedsDespawned, compute_totally_accurate_global_transform},
    physics::{location::Location, player_world::PlayerWorld, structure_physics::ChunkPhysicsPart},
    projectiles::{
        causer::Causer,
        missile::{Explosion, ExplosionSystemSet},
    },
    registry::Registry,
    structure::{
        Structure,
        block_health::events::{BlockDestroyedMessage, BlockTakeDamageMessage},
        coordinates::{BlockCoordinate, UnboundBlockCoordinate, UnboundCoordinateType},
        shields::Shield,
    },
};

use crate::{netty::sync::sync_bodies::DontNotifyClientOfDespawn, structure::shared::MeltingDownSet};

/// 1 unit of explosion power = this amount of health. Bigger this number is, the more damage explosives will do.
const HEALTH_PER_EXPLOSION_POWER: f32 = 8.0;

#[derive(Message, Debug)]
/// This event is sent whenever an explosion hits an entity
///
/// Currently this **isn't** sent for structures being hit, but it will be in the future.
pub struct ExplosionHitMessage {
    /// The explosion that caused this event
    pub explosion: Explosion,
    /// The explosion's location
    pub explosion_location: Location,
    /// The entity that was hit by the explosion
    pub hit_entity: Entity,
}

fn respond_to_explosion(
    mut commands: Commands,
    q_explosions: Query<(Entity, &Location, &RapierContextEntityLink, &Explosion, Option<&Causer>), Added<Explosion>>,
    q_player_world: Query<&Location, With<PlayerWorld>>,
    q_excluded: Query<(), Or<(With<Explosion>, Without<Collider>)>>,

    mut q_structure: Query<(&Location, &mut Structure)>,
    context_access: ReadRapierContext,

    q_chunk: Query<&ChunkPhysicsPart>,
    blocks_registry: Res<Registry<Block>>,
    mut evw_block_take_damage: MessageWriter<BlockTakeDamageMessage>,
    mut evw_block_destroyed: MessageWriter<BlockDestroyedMessage>,
    mut ev_writer_explosion_hit: MessageWriter<ExplosionHitMessage>,
    // blocks: Res<Registry<Block>>,
    // mut evw_bc: MessageWriter<BlockChangedMessage>,
    q_shield: Query<&Shield>,
    q_trans: Query<(&Transform, Option<&ChildOf>)>,
) {
    for (ent, &explosion_loc, physics_world, &explosion, causer) in q_explosions.iter() {
        info!("Found explosion @ {explosion_loc}!");
        commands.entity(ent).insert((NeedsDespawned, DontNotifyClientOfDespawn));

        let Ok(player_world_loc) = q_player_world.get(physics_world.0) else {
            continue;
        };

        // for (loc, trans, mut s) in q_structures.iter_mut() {
        //     info!("EXPLOSION: {}; STRUCTURE: {}", explosion_loc, loc);
        //     let rel = trans.rotation.inverse() * (explosion_loc - *loc).absolute_coords_f32();
        //     let Ok(c) = BlockCoordinate::try_from(s.relative_coords_to_local_coords(rel.x, rel.y, rel.z)) else {
        //         continue;
        //     };
        //     s.set_block_at(
        //         c,
        //         blocks.from_id("cosmos:stone").unwrap(),
        //         Default::default(),
        //         &blocks,
        //         Some(&mut evw_bc),
        //     );
        // }

        let max_radius = explosion.power.sqrt();

        // Have to do this by hand because `GlobalTransform` doesn't have enough time to
        // propagate down
        let explosion_rapier_coordinates = (explosion_loc - *player_world_loc).absolute_coords_f32();

        let mut hits = vec![];

        let context = context_access.get(*physics_world);

        context.intersections_with_shape(
            explosion_rapier_coordinates,
            Quat::IDENTITY,
            &Collider::ball(max_radius),
            QueryFilter::default().exclude_collider(ent).predicate(&|x| !q_excluded.contains(x)),
            |hit_entity| {
                hits.push(hit_entity);

                true
            },
        );

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
            let Ok((structure_loc, mut structure)) = q_structure.get_mut(hit) else {
                ev_writer_explosion_hit.write(ExplosionHitMessage {
                    explosion,
                    explosion_location: explosion_loc,
                    hit_entity: hit,
                });

                continue;
            };
            let Some(structure_g_trans) = compute_totally_accurate_global_transform(hit, &q_trans) else {
                error!("Error computing gtrans for {hit:?}!");
                continue;
            };
            let explosion_relative_position =
                structure_g_trans.affine().inverse().matrix3 * (explosion_loc - *structure_loc).absolute_coords_f32();

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
                .filter(|&coords| structure.has_block_at(coords)) // Remove this once `include_air` works.
                .flat_map(|this_block| {
                    calculate_block_explosion_power(
                        &structure,
                        this_block,
                        explosion_relative_position,
                        &explosion,
                        &blocks_registry,
                        max_radius_sqrd,
                    )
                })
                .collect::<Vec<(BlockCoordinate, f32)>>();

            for (block, explosion_power) in hit_blocks {
                let block_coord = structure_g_trans
                    .mul_transform(Transform::from_translation(structure.block_relative_position(block)))
                    .translation();

                // Ensure no shield is hit before breaking block
                // This ray will only find shields
                if context
                    .cast_ray(
                        block_coord,
                        explosion_rapier_coordinates - block_coord,
                        1.0,
                        true,
                        QueryFilter::default().predicate(&|e| q_shield.get(e).map(|s| s.is_enabled()).unwrap_or(false)),
                    )
                    .is_none()
                {
                    structure.block_take_damage(
                        block,
                        &blocks_registry,
                        explosion_power * HEALTH_PER_EXPLOSION_POWER,
                        Some((&mut evw_block_take_damage, &mut evw_block_destroyed)),
                        causer.map(|x| x.0),
                    );
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

pub(super) fn register(app: &mut App) {
    app.add_message::<ExplosionHitMessage>();

    app.add_systems(
        FixedUpdate,
        respond_to_explosion
            .ambiguous_with(MeltingDownSet::ProcessMeltingDown)
            .in_set(ExplosionSystemSet::ProcessExplosions)
            .ambiguous_with(BlockMessagesSet::SendMessagesForNextFrame), // Order of blocks being updated doesn't matter
                                                                     // .after(ShieldSet::RechargeShields)
                                                                     // .after(FixedUpdateSet::LocationSyncing)
                                                                     // .before(FixedUpdateSet::PrePhysics)
                                                                     // .before(ShieldSet::OnShieldHit), // .in_set(BlockHealthSet::SendHealthChanges),
    );
}
