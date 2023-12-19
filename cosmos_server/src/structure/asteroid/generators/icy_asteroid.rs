use bevy::{prelude::*, tasks::AsyncComputeTaskPool, utils::HashMap};
use cosmos_core::{
    block::{Block, BlockFace},
    physics::location::Location,
    registry::ReadOnlyRegistry,
    structure::{
        asteroid::loading::AsteroidNeedsCreated,
        block_storage::BlockStorer,
        chunk::Chunk,
        coordinates::{BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate},
        loading::StructureLoadingSet,
        Structure,
    },
    utils::timer::UtilsTimer,
};
use noise::NoiseFn;

use crate::{
    init::init_world::ReadOnlyNoise,
    state::GameState,
    structure::{asteroid::generator::GeneratingAsteroids, planet::biosphere::TemperatureRange},
};

use super::{register_asteroid_generator, AsteroidGeneratorComponent};

#[derive(Clone, Copy, Component, Default)]
struct IcyAsteroidMarker;

impl AsteroidGeneratorComponent for IcyAsteroidMarker {}

fn start_generating_asteroid(
    q_asteroids_need_generated: Query<(Entity, &Structure, &Location), (With<AsteroidNeedsCreated>, With<IcyAsteroidMarker>)>,
    noise: Res<ReadOnlyNoise>,
    blocks: Res<ReadOnlyRegistry<Block>>,
    mut commands: Commands,
    mut generating_asteroids: ResMut<GeneratingAsteroids>,
) {
    for (structure_entity, structure, loc) in q_asteroids_need_generated.iter() {
        let (local_x, local_y, local_z) = (loc.local.x as f64, loc.local.y as f64, loc.local.z as f64);

        let (bx, by, bz) = structure.block_dimensions().into();

        let noise = noise.clone();

        let thread_pool = AsyncComputeTaskPool::get();

        let blocks = blocks.clone();

        let task = thread_pool.spawn(async move {
            let noise = noise.inner();

            let distance_threshold = (bz as f64 / 4.0 * (noise.get([local_x, local_y, local_z]).abs() + 1.0).min(25.0)) as f32;

            let timer = UtilsTimer::start();

            let blocks = blocks.registry();
            let stone = blocks.from_id("cosmos:stone").expect("Missing cosmos:stone");
            let ore = blocks.from_id("cosmos:test_ore").expect("Missing text ore");

            let mut chunks = HashMap::new();

            for z in 0..bz {
                for y in 0..by {
                    for x in 0..bx {
                        let x_pos = x as f32 - bx as f32 / 2.0;
                        let y_pos = y as f32 - by as f32 / 2.0;
                        let z_pos = z as f32 - bz as f32 / 2.0;

                        let noise_here = (noise.get([
                            x_pos as f64 * 0.03 + local_x,
                            y_pos as f64 * 0.03 + local_y,
                            z_pos as f64 * 0.03 + local_z,
                        ]) * 150.0) as f32;

                        let dist = x_pos * x_pos + y_pos * y_pos + z_pos * z_pos + noise_here * noise_here;

                        let distance_threshold = distance_threshold + noise_here / 3.0;

                        if dist < distance_threshold * distance_threshold {
                            let coords = BlockCoordinate::new(x, y, z);
                            let chunk_coords = ChunkCoordinate::for_block_coordinate(coords);
                            let chunk_block_coords = ChunkBlockCoordinate::for_block_coordinate(coords);

                            const RANDOM_OFFSET: f64 = 2378.0;

                            let ore_noise = noise.get([
                                x_pos as f64 * 0.03 + local_x + RANDOM_OFFSET,
                                y_pos as f64 * 0.03 + local_y + RANDOM_OFFSET,
                                z_pos as f64 * 0.03 + local_z + RANDOM_OFFSET,
                            ]);

                            let block = if ore_noise > 0.1 { ore } else { stone };

                            chunks.entry(chunk_coords).or_insert_with(|| Chunk::new(chunk_coords)).set_block_at(
                                chunk_block_coords,
                                block,
                                BlockFace::Top,
                            );
                        }
                    }
                }
            }

            timer.log_duration(&format!("Asteroid {bx}x{by}x{bz} generation time: {bx}:"));

            chunks.into_iter().map(|(_, c)| c).collect::<Vec<Chunk>>()
        });

        generating_asteroids.add_generating_asteroid(structure_entity, task, &mut commands);
    }
}

pub(super) fn register(app: &mut App) {
    register_asteroid_generator::<IcyAsteroidMarker>(app, "cosmos:icy", TemperatureRange::new(0.0, 1000000.0));

    app.add_systems(
        Update,
        start_generating_asteroid
            .in_set(StructureLoadingSet::LoadStructure)
            .run_if(in_state(GameState::Playing)),
    );
}
