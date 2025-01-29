use bevy::{prelude::*, tasks::AsyncComputeTaskPool, utils::HashMap};
use cosmos_core::{
    block::{block_rotation::BlockRotation, Block},
    physics::location::Location,
    registry::ReadOnlyRegistry,
    state::GameState,
    structure::{
        block_storage::BlockStorer,
        chunk::Chunk,
        coordinates::{BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate},
        Structure,
    },
    utils::timer::UtilsTimer,
};
use noise::NoiseFn;
use rand::Rng;

use crate::{
    init::init_world::{ReadOnlyNoise, ServerSeed},
    rng::get_rng_for_sector,
    structure::{
        asteroid::generator::{AsteroidGenerationSet, GenerateAsteroidEvent, GeneratingAsteroids},
        planet::biosphere::TemperatureRange,
    },
};

use super::{register_asteroid_generator, AsteroidGeneratorComponent};

#[derive(Debug, Clone)]
pub struct AsteroidOreEntry {
    pub ore: &'static str,
    /// 1.0 = default ore patch size
    pub size: f32,
    /// 1.0 = common
    pub rarity: f32,
}

pub fn register_standard_asteroid_generation<T: AsteroidGeneratorComponent>(
    app: &mut App,
    id: &'static str,
    temperature_range: TemperatureRange,
    ore_ids: Vec<AsteroidOreEntry>,
    rock_id: &'static str,
) {
    register_asteroid_generator::<T>(app, id, temperature_range);

    let start_generating_molten_asteroid =
        move |q_molten_asteroids: Query<(Entity, &Structure, &Location), With<T>>,
              mut ev_reader: EventReader<GenerateAsteroidEvent>,
              noise: Res<ReadOnlyNoise>,
              blocks: Res<ReadOnlyRegistry<Block>>,
              server_seed: Res<ServerSeed>,
              mut generating_asteroids: ResMut<GeneratingAsteroids>| {
            for ent in ev_reader.read() {
                let Ok((structure_entity, structure, loc)) = q_molten_asteroids.get(ent.0) else {
                    continue;
                };

                let (local_x, local_y, local_z) = (loc.local.x as f64, loc.local.y as f64, loc.local.z as f64);

                let (bx, by, bz) = structure.block_dimensions().into();

                let noise = noise.clone();

                let thread_pool = AsyncComputeTaskPool::get();

                let blocks = blocks.clone();

                let ore_ids = ore_ids.clone();
                let mut s_rng = get_rng_for_sector(&server_seed, &loc.sector());
                let offsets = ore_ids.iter().map(|_| s_rng.gen::<f64>() * 10000.0).collect::<Vec<_>>();

                let task = thread_pool.spawn(async move {
                    let noise = noise.inner();

                    let distance_threshold = (bz as f64 / 4.0 * (noise.get([local_x, local_y, local_z]).abs() + 1.0).min(25.0)) as f32;

                    let timer = UtilsTimer::start();

                    let blocks = blocks.registry();
                    let stone = blocks.from_id(rock_id).unwrap_or_else(|| panic!("Missing block {rock_id}"));
                    let ore_blocks = ore_ids
                        .iter()
                        .map(|x| {
                            (
                                blocks.from_id(x.ore).unwrap_or_else(|| panic!("Missing block {}", x.ore)),
                                x.rarity,
                                1.0 / x.size,
                            )
                        })
                        .collect::<Vec<_>>();

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

                                    let max_noise = ore_blocks
                                        .iter()
                                        .zip(offsets.iter())
                                        .map(|(&(block, rarity, size), &offset)| {
                                            (
                                                block,
                                                noise.get([
                                                    x_pos as f64 * size as f64 * 0.1 + local_x + offset,
                                                    y_pos as f64 * size as f64 * 0.1 + local_y + offset,
                                                    z_pos as f64 * size as f64 * 0.1 + local_z + offset,
                                                ]) * rarity as f64,
                                            )
                                        })
                                        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                                    let block = if let Some((ore_block, max_noise)) = max_noise {
                                        if max_noise < 0.1 {
                                            stone
                                        } else {
                                            ore_block
                                        }
                                    } else {
                                        stone
                                    };

                                    chunks.entry(chunk_coords).or_insert_with(|| Chunk::new(chunk_coords)).set_block_at(
                                        chunk_block_coords,
                                        block,
                                        BlockRotation::default(),
                                    );
                                }
                            }
                        }
                    }

                    timer.log_duration(&format!("Asteroid {bx}x{by}x{bz} generation time: {bx}:"));

                    chunks.into_iter().map(|(_, c)| c).collect::<Vec<Chunk>>()
                });

                generating_asteroids.add_generating_asteroid(structure_entity, task);
            }
        };

    app.add_systems(
        Update,
        start_generating_molten_asteroid
            .in_set(AsteroidGenerationSet::GenerateAsteroid)
            .ambiguous_with(AsteroidGenerationSet::GenerateAsteroid)
            .run_if(in_state(GameState::Playing)),
    );
}
