use bevy::{
    ecs::{
        event::EventWriter,
        query::{Added, Or, Without},
        schedule::IntoSystemConfigs,
    },
    math::Quat,
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
        coordinates::{UnboundBlockCoordinate, UnboundCoordinateType},
        structure_block::StructureBlock,
        Structure,
    },
};

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

        println!("Hits: {hits:?}");

        let mut ents = HashSet::new();
        for ent in hits {
            if let Ok(chunk_ent) = q_chunk.get(ent) {
                ents.insert(chunk_ent.structure_entity);
            } else {
                ents.insert(ent);
            }
        }

        let max_block_radius = max_radius.ceil() as UnboundCoordinateType;

        for &hit in ents.iter() {
            if let Ok((structure_g_trans, mut structure)) = q_structure.get_mut(hit) {
                let relative_position =
                    structure_g_trans.affine().inverse().matrix3 * (explosion_g_trans.translation() - structure_g_trans.translation());

                let local_coords = structure.relative_coords_to_local_coords(relative_position.x, relative_position.y, relative_position.z);

                println!("{relative_position} => {local_coords} +- {max_block_radius}");

                let blocks = structure
                    .block_iter(
                        local_coords - UnboundBlockCoordinate::splat(max_block_radius),
                        local_coords + UnboundBlockCoordinate::splat(max_block_radius),
                        true, // Include air false is broken for some reason
                    )
                    .collect::<Vec<StructureBlock>>();

                for block in blocks {
                    structure.remove_block_at(block.coords(), &blocks_registry, Some(&mut ev_writer));
                }

                // structure.set_block_at(
                //     BlockCoordinate::try_from(local_coords).unwrap(),
                //     blocks_registry.from_id("cosmos:glass").unwrap(),
                //     Default::default(),
                //     &blocks_registry,
                //     Some(&mut ev_writer),
                // );
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, respond_to_explosion.in_set(ExplosionSystemSet::ProcessExplosions));
}
