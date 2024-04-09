use bevy::{
    ecs::{
        event::EventWriter,
        query::{Added, Or, Without},
        schedule::IntoSystemConfigs,
    },
    log::info,
    math::{Quat, Vec3},
    prelude::{App, Commands, Entity, Query, Res, Update, With},
    transform::components::GlobalTransform,
    utils::HashSet,
};
use bevy_rapier3d::{
    geometry::{Collider, Sensor},
    pipeline::QueryFilter,
    plugin::RapierContext,
    prelude::PhysicsWorld,
};

use cosmos_core::{
    block::Block,
    ecs::NeedsDespawned,
    events::block_events::BlockChangedEvent,
    projectiles::missile::{Explosion, ExplosionSystemSet},
    registry::Registry,
    structure::{
        chunk::ChunkEntity,
        coordinates::{BlockCoordinate, UnboundBlockCoordinate, UnboundCoordinateType},
        Structure,
    },
};

/// 1 unit of explosion power = this amount of health. Bigger this number is, the more damage explosives will do.
const HEALTH_PER_EXPLOSION_POWER: f32 = 8.0;

fn respond_to_explosion(
    mut commands: Commands,
    q_explosions: Query<(Entity, &GlobalTransform, Option<&PhysicsWorld>, &Explosion), Added<Explosion>>,
    q_excluded: Query<(), Or<(With<Explosion>, Without<Collider>, With<Sensor>)>>,

    mut q_structure: Query<(&GlobalTransform, &mut Structure)>,
    context: Res<RapierContext>,

    q_chunk: Query<&ChunkEntity>,
    blocks_registry: Res<Registry<Block>>,
    mut ev_writer: EventWriter<BlockChangedEvent>,
) {
    for (ent, explosion_g_trans, physics_world, explosion) in q_explosions.iter() {
        commands.entity(ent).insert(NeedsDespawned);

        let max_radius = explosion.power.sqrt();

        let physics_world = physics_world.copied().unwrap_or_default();

        let mut hits = vec![];

        context
            .intersections_with_shape(
                physics_world.world_id,
                explosion_g_trans.translation(),
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
            let Ok((structure_g_trans, mut structure)) = q_structure.get_mut(hit) else {
                continue;
            };
            let explosion_relative_position =
                structure_g_trans.affine().inverse().matrix3 * (explosion_g_trans.translation() - structure_g_trans.translation());

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
                // .filter(|&this_block| {
                //     structure
                //         .block_relative_position(this_block)
                //         .distance_squared(explosion_relative_position)
                //         <= max_radius_sqrd
                // })
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

    remaining_explosion_power = remaining_explosion_power * (1.0 - decay_percent);

    if remaining_explosion_power > 0.0 {
        Some((this_block, remaining_explosion_power))
    } else {
        None
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, respond_to_explosion.in_set(ExplosionSystemSet::ProcessExplosions));
}
