//! Responsible for the default generation of biospheres.

use std::{marker::PhantomData, mem::swap};

use bevy::{
    prelude::{
        warn, Commands, Component, DespawnRecursiveExt, Entity, Event, EventReader, EventWriter, Query, Res, ResMut, Resource, With,
    },
    tasks::AsyncComputeTaskPool,
};
use cosmos_core::{
    block::{Block, BlockFace},
    netty::cosmos_encoder,
    physics::location::Location,
    registry::Registry,
    structure::{
        block_storage::BlockStorer,
        chunk::{Chunk, CHUNK_DIMENSIONS},
        coordinates::{BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate, CoordinateType},
        lod::{LodDelta, LodNetworkMessage, SetLodMessage},
        lod_chunk::LodChunk,
        planet::{ChunkFaces, Planet},
        Structure,
    },
    utils::array_utils::flatten_2d,
};
use futures_lite::future;
use noise::NoiseFn;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::{
    init::init_world::{Noise, ReadOnlyNoise},
    structure::planet::lods::generate_lods::{
        AsyncGeneratingLod, DoneGeneratingLod, GeneratingLod, GeneratingLods, LodNeedsGeneratedForPlayer,
    },
};

use super::{GeneratingChunk, GeneratingChunks, TGenerateChunkEvent};

/// Tells the chunk to generate its features.
#[derive(Debug, Event)]
pub struct GenerateChunkFeaturesEvent<T: Component> {
    _phantom: PhantomData<T>,
    /// cx, cy, cz.
    pub chunk_coords: ChunkCoordinate,
    /// The structure entity that contains this chunk.
    pub structure_entity: Entity,
}

/// Gets the "y" value of a block on the planet. This "y" value is relative to the face the block is on.
///
/// * `noise_generator` Used to generate noise values. Seeded for this world seed.
/// * `(x, y, z)` Block x/y/z in the structure
/// * `(structure_x, structure_y, structure_z)` Where the structure is in the universe - used to offset the noise values so no two structures are the same.
/// * `(middle_air_start)` The midpoint of the extremes of heights. Aka if noise generates 0, then this should return middle_air_start.
/// * `amplitude` Value passed in by the `GenerationParemeters`. Represents how tall the terrain will be
/// * `delta` Value passed in by the `GenerationParemeters`. Represents how much each change in x/y/z will effect the terrain. Small values = lesser effect
/// * `iterations` Value passed in by the `GenerationParemeters`. Represents how many times the noise function will be run
fn get_block_height(
    noise_generator: &noise::OpenSimplex,
    block_coords: BlockCoordinate,
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    middle: CoordinateType,
    amplitude: f64,
    delta: f64,
    iterations: usize,
) -> f64 {
    let mut depth: f64 = 0.0;
    for iteration in 1..=iterations {
        let iteration = iteration as f64;
        depth += noise_generator.get([
            (block_coords.x as f64 + structure_x) * (delta / iteration),
            (block_coords.y as f64 + structure_y) * (delta / iteration),
            (block_coords.z as f64 + structure_z) * (delta / iteration),
        ]) * amplitude
            * iteration;
    }

    middle as f64 + depth
}

/// Sends a ChunkInitEvent for every chunk that's done generating, monitors when chunks are finished generating.
pub fn notify_when_done_generating_terrain<T: Component>(
    mut generating: ResMut<GeneratingChunks<T>>,
    mut event_writer: EventWriter<GenerateChunkFeaturesEvent<T>>,
    mut structure_query: Query<&mut Structure>,
) {
    let mut still_todo = Vec::with_capacity(generating.generating.len());

    swap(&mut generating.generating, &mut still_todo);

    for mut generating_chunk in still_todo {
        if let Some(chunks) = future::block_on(future::poll_once(&mut generating_chunk.task)) {
            let (chunk, structure_entity) = chunks;

            if let Ok(mut structure) = structure_query.get_mut(structure_entity) {
                let chunk_coords = chunk.chunk_coordinates();

                structure.set_chunk(chunk);

                event_writer.send(GenerateChunkFeaturesEvent::<T> {
                    _phantom: PhantomData,
                    structure_entity,
                    chunk_coords,
                });
            }
        } else {
            generating.generating.push(generating_chunk);
        }
    }
}

#[inline]
fn generate_face_chunk<S: BiosphereGenerationStrategy, T: Component + Clone + Default, C: BlockStorer>(
    block_coords: BlockCoordinate,
    structure_coords: (f64, f64, f64),
    s_dimensions: CoordinateType,
    noise_generator: &noise::OpenSimplex,
    block_ranges: &BlockLayers<T>,
    chunk: &mut C,
    up: BlockFace,
    scale: CoordinateType,
) {
    let (sx, sy, sz) = (block_coords.x, block_coords.y, block_coords.z);

    for i in 0..CHUNK_DIMENSIONS {
        for j in 0..CHUNK_DIMENSIONS {
            let seed_coords: BlockCoordinate = match up {
                BlockFace::Top => (sx + i * scale, s_dimensions, sz + j * scale),
                BlockFace::Bottom => (sx + i * scale, 0, sz + j * scale),
                BlockFace::Front => (sx + i * scale, sy + j * scale, s_dimensions),
                BlockFace::Back => (sx + i * scale, sy + j * scale, 0),
                BlockFace::Right => (s_dimensions, sy + i * scale, sz + j * scale),
                BlockFace::Left => (0, sy + i * scale, sz + j * scale),
            }
            .into();

            let mut height = s_dimensions;
            let mut concrete_ranges = Vec::new();
            for (block, level) in block_ranges.ranges.iter() {
                let level_top = S::get_top_height(
                    up,
                    seed_coords,
                    structure_coords,
                    s_dimensions,
                    noise_generator,
                    height - level.middle_depth,
                    level.amplitude,
                    level.delta,
                    level.iterations,
                );
                concrete_ranges.push((block, level_top));
                height = level_top;
            }

            for chunk_height in 0..CHUNK_DIMENSIONS {
                let coords: ChunkBlockCoordinate = match up {
                    BlockFace::Front | BlockFace::Back => (i, j, chunk_height),
                    BlockFace::Top | BlockFace::Bottom => (i, chunk_height, j),
                    BlockFace::Right | BlockFace::Left => (chunk_height, i, j),
                }
                .into();

                let height = match up {
                    BlockFace::Front => sz + chunk_height * scale,
                    BlockFace::Back => s_dimensions - (sz + chunk_height * scale),
                    BlockFace::Top => sy + chunk_height * scale,
                    BlockFace::Bottom => s_dimensions - (sy + chunk_height * scale),
                    BlockFace::Right => sx + chunk_height * scale,
                    BlockFace::Left => s_dimensions - (sx + chunk_height * scale),
                };

                let block = block_ranges.face_block(height, &concrete_ranges, block_ranges.sea_level, block_ranges.sea_block(), scale);
                if let Some(block) = block {
                    chunk.set_block_at(coords, block, up);
                }
                // else if scale != 1 {
                //     let below_coords = match up {
                //         BlockFace::Front => (coords.x, coords.y, coords.z - 1),
                //         BlockFace::Back => (coords.x, coords.y, coords.z + 1),
                //         BlockFace::Top => (coords.x, coords.y - 1, coords.z),
                //         BlockFace::Bottom => (coords.x, coords.y + 1, coords.z),
                //         BlockFace::Right => (coords.x - 1, coords.y, coords.z),
                //         BlockFace::Left => (coords.x + 1, coords.y, coords.z),
                //     }
                //     .into();

                //     if let Some((candidate, _)) = concrete_ranges.iter().find(|(_, h)| height + scale > *h) {
                //         chunk.set_block_at(below_coords, candidate, up);
                //     }
                // }
            }
        }
    }
}

fn generate_edge_chunk<S: BiosphereGenerationStrategy, T: Component + Clone + Default, C: BlockStorer>(
    block_coords: BlockCoordinate,
    structure_coords: (f64, f64, f64),
    s_dimensions: CoordinateType,
    noise_generator: &noise::OpenSimplex,
    block_ranges: &BlockLayers<T>,
    chunk: &mut C,
    j_up: BlockFace,
    k_up: BlockFace,
    scale: CoordinateType,
) {
    for i in 0..CHUNK_DIMENSIONS {
        let i_scaled = i * scale;
        let mut j_layers_cache: Vec<Vec<(&Block, CoordinateType)>> = vec![vec![]; CHUNK_DIMENSIONS as usize];
        for (j, j_layers) in j_layers_cache.iter_mut().enumerate() {
            let j_scaled = j as CoordinateType * scale;

            // Seed coordinates and j-direction noise functions.
            let (mut x, mut y, mut z) = (block_coords.x + i_scaled, block_coords.y + i_scaled, block_coords.z + i_scaled);

            match j_up {
                BlockFace::Front => z = s_dimensions,
                BlockFace::Back => z = 0,
                BlockFace::Top => y = s_dimensions,
                BlockFace::Bottom => y = 0,
                BlockFace::Right => x = s_dimensions,
                BlockFace::Left => x = 0,
            };
            match k_up {
                BlockFace::Front | BlockFace::Back => z = block_coords.z + j_scaled,
                BlockFace::Top | BlockFace::Bottom => y = block_coords.y + j_scaled,
                BlockFace::Right | BlockFace::Left => x = block_coords.x + j_scaled,
            };
            let mut height = s_dimensions;
            for (block, layer) in block_ranges.ranges.iter() {
                let layer_top = S::get_top_height(
                    j_up,
                    BlockCoordinate::new(x, y, z),
                    structure_coords,
                    s_dimensions,
                    noise_generator,
                    height - layer.middle_depth,
                    layer.amplitude,
                    layer.delta,
                    layer.iterations,
                );
                j_layers.push((block, layer_top));
                height = layer_top;
            }
        }

        // The minimum (j, j) on the 45 where the two top heights intersect.
        let mut first_both_45 = s_dimensions;
        for j in 0..CHUNK_DIMENSIONS {
            let j_scaled = j as CoordinateType * scale;

            // Seed coordinates and k-direction noise functions.
            let (mut x, mut y, mut z) = (block_coords.x + i_scaled, block_coords.y + i_scaled, block_coords.z + i_scaled);
            match k_up {
                BlockFace::Front => z = s_dimensions,
                BlockFace::Back => z = 0,
                BlockFace::Top => y = s_dimensions,
                BlockFace::Bottom => y = 0,
                BlockFace::Right => x = s_dimensions,
                BlockFace::Left => x = 0,
            };
            match j_up {
                BlockFace::Front | BlockFace::Back => z = block_coords.z + j_scaled,
                BlockFace::Top | BlockFace::Bottom => y = block_coords.y + j_scaled,
                BlockFace::Right | BlockFace::Left => x = block_coords.x + j_scaled,
            };
            let j_height = match j_up {
                BlockFace::Front => z,
                BlockFace::Back => s_dimensions - z,
                BlockFace::Top => y,
                BlockFace::Bottom => s_dimensions - y,
                BlockFace::Right => x,
                BlockFace::Left => s_dimensions - x,
            };

            let mut height = s_dimensions;
            let mut k_layers: Vec<(&Block, CoordinateType)> = vec![];
            for (block, layer) in block_ranges.ranges.iter() {
                let layer_top = S::get_top_height(
                    k_up,
                    BlockCoordinate::new(x, y, z),
                    structure_coords,
                    s_dimensions,
                    noise_generator,
                    height - layer.middle_depth,
                    layer.amplitude,
                    layer.delta,
                    layer.iterations,
                );
                k_layers.push((block, layer_top));
                height = layer_top;
            }

            if j_layers_cache[j as usize][0].1 == j_height && k_layers[0].1 == j_height && first_both_45 == s_dimensions {
                first_both_45 = j_height;
            }

            for (k, j_layers) in j_layers_cache.iter().enumerate() {
                let mut chunk_block_coords = ChunkBlockCoordinate::new(i, i, i);
                match j_up {
                    BlockFace::Front | BlockFace::Back => chunk_block_coords.z = j,
                    BlockFace::Top | BlockFace::Bottom => chunk_block_coords.y = j,
                    BlockFace::Right | BlockFace::Left => chunk_block_coords.x = j,
                };
                match k_up {
                    BlockFace::Front | BlockFace::Back => chunk_block_coords.z = k as CoordinateType,
                    BlockFace::Top | BlockFace::Bottom => chunk_block_coords.y = k as CoordinateType,
                    BlockFace::Right | BlockFace::Left => chunk_block_coords.x = k as CoordinateType,
                };

                let k_height = match k_up {
                    BlockFace::Front => block_coords.z + chunk_block_coords.z * scale,
                    BlockFace::Back => s_dimensions - (block_coords.z + chunk_block_coords.z * scale),
                    BlockFace::Top => block_coords.y + chunk_block_coords.y * scale,
                    BlockFace::Bottom => s_dimensions - (block_coords.y + chunk_block_coords.y * scale),
                    BlockFace::Right => block_coords.x + chunk_block_coords.x * scale,
                    BlockFace::Left => s_dimensions - (block_coords.x + chunk_block_coords.x * scale),
                };

                if j_height < first_both_45 || k_height < first_both_45 {
                    // The top block needs different "top" to look good, the block can't tell which "up" looks good.
                    let block_up = Planet::get_planet_face_without_structure(
                        BlockCoordinate::new(
                            block_coords.x + chunk_block_coords.x * scale,
                            block_coords.y + chunk_block_coords.y * scale,
                            block_coords.z + chunk_block_coords.z * scale,
                        ),
                        s_dimensions,
                    );
                    let block = block_ranges.edge_block(
                        j_height,
                        k_height,
                        j_layers,
                        &k_layers,
                        block_ranges.sea_level,
                        block_ranges.sea_block(),
                        scale,
                    );
                    if let Some(block) = block {
                        chunk.set_block_at(chunk_block_coords, block, block_up);
                    }
                }
            }
        }
    }
}

// Might trim 45s, see generate_edge_chunk.
fn generate_corner_chunk<S: BiosphereGenerationStrategy, T: Component + Clone + Default, C: BlockStorer>(
    block_coords: BlockCoordinate,
    structure_coords: (f64, f64, f64),
    s_dimensions: CoordinateType,
    noise_generator: &noise::OpenSimplex,
    block_ranges: &BlockLayers<T>,
    chunk: &mut C,
    x_up: BlockFace,
    y_up: BlockFace,
    z_up: BlockFace,
    scale: CoordinateType,
) {
    // x top height cache.
    let mut x_layers: Vec<Vec<(&Block, CoordinateType)>> = vec![vec![]; CHUNK_DIMENSIONS as usize * CHUNK_DIMENSIONS as usize];
    for j in 0..CHUNK_DIMENSIONS {
        let j_scaled = j * scale;
        for k in 0..CHUNK_DIMENSIONS {
            let k_scaled = k * scale;

            let index = flatten_2d(j as usize, k as usize, CHUNK_DIMENSIONS as usize);

            // Seed coordinates for the noise function.
            let seed_coords = match x_up {
                BlockFace::Right => (s_dimensions, block_coords.y + j_scaled, block_coords.z + k_scaled),
                _ => (0, block_coords.y + j_scaled, block_coords.z + k_scaled),
            }
            .into();

            // Unmodified top height.
            let mut height = s_dimensions;
            for (block, level) in block_ranges.ranges.iter() {
                let level_top = S::get_top_height(
                    x_up,
                    seed_coords,
                    structure_coords,
                    s_dimensions,
                    noise_generator,
                    height - level.middle_depth,
                    level.amplitude,
                    level.delta,
                    level.iterations,
                );
                x_layers[index].push((block, level_top));
                height = level_top;
            }
        }
    }

    // y top height cache.
    let mut y_layers: Vec<Vec<(&Block, CoordinateType)>> = vec![vec![]; CHUNK_DIMENSIONS as usize * CHUNK_DIMENSIONS as usize];
    for i in 0..CHUNK_DIMENSIONS {
        let i_scaled = i * scale;
        for k in 0..CHUNK_DIMENSIONS {
            let k_scaled = k * scale;

            let index = flatten_2d(i as usize, k as usize, CHUNK_DIMENSIONS as usize);

            // Seed coordinates for the noise function. Which loop variable goes to which xyz must agree everywhere.
            let seed_coords = match y_up {
                BlockFace::Top => (block_coords.x + i_scaled, s_dimensions, block_coords.z + k_scaled),
                _ => (block_coords.x + i_scaled, 0, block_coords.z + k_scaled),
            }
            .into();

            // Unmodified top height.
            let mut height = s_dimensions;
            for (block, level) in block_ranges.ranges.iter() {
                let level_top = S::get_top_height(
                    y_up,
                    seed_coords,
                    structure_coords,
                    s_dimensions,
                    noise_generator,
                    height - level.middle_depth,
                    level.amplitude,
                    level.delta,
                    level.iterations,
                );
                y_layers[index].push((block, level_top));
                height = level_top;
            }
        }
    }

    for i in 0..CHUNK_DIMENSIONS {
        let i_scaled = i * scale;
        for j in 0..CHUNK_DIMENSIONS {
            let j_scaled = j * scale;

            // Seed coordinates for the noise function.
            let seed_coords = match z_up {
                BlockFace::Front => (block_coords.x + i_scaled, block_coords.y + j_scaled, s_dimensions),
                _ => (block_coords.x + i_scaled, block_coords.y + j_scaled, 0),
            }
            .into();

            // Unmodified top height.
            let mut height = s_dimensions;
            let mut z_layers = vec![];
            for (block, level) in block_ranges.ranges.iter() {
                let level_top = S::get_top_height(
                    z_up,
                    seed_coords,
                    structure_coords,
                    s_dimensions,
                    noise_generator,
                    height - level.middle_depth,
                    level.amplitude,
                    level.delta,
                    level.iterations,
                );
                z_layers.push((block, level_top));
                height = level_top;
            }

            for k in 0..CHUNK_DIMENSIONS {
                let k_scaled = k * scale;

                let z_height = match z_up {
                    BlockFace::Front => block_coords.z + k_scaled,
                    _ => s_dimensions - (block_coords.z + k_scaled),
                };
                let y_height = match y_up {
                    BlockFace::Top => block_coords.y + j_scaled,
                    _ => s_dimensions - (block_coords.y + j_scaled),
                };
                let x_height = match x_up {
                    BlockFace::Right => block_coords.x + i_scaled,
                    _ => s_dimensions - (block_coords.x + i_scaled),
                };

                let block_up = Planet::get_planet_face_without_structure(
                    BlockCoordinate::new(block_coords.x + i_scaled, block_coords.y + j_scaled, block_coords.z + k_scaled),
                    s_dimensions,
                );
                let block = block_ranges.corner_block(
                    x_height,
                    y_height,
                    z_height,
                    &x_layers[flatten_2d(j as usize, k as usize, CHUNK_DIMENSIONS as usize)],
                    &y_layers[flatten_2d(i as usize, k as usize, CHUNK_DIMENSIONS as usize)],
                    &z_layers,
                    block_ranges.sea_level,
                    block_ranges.sea_block(),
                    scale,
                );
                if let Some(block) = block {
                    chunk.set_block_at(ChunkBlockCoordinate::new(i, j, k), block, block_up);
                }
            }
        }
    }
}

const GUIDE_MIN: CoordinateType = 100;
/// Used to change the algorithm used for base terrain generation.
///
/// Try tweaking the values of GenerationParemeters first before making your own custom generation function.
///
/// For most cases, the `DefaultBiosphereGenerationStrategy` strategy will work.
pub trait BiosphereGenerationStrategy {
    /// Gets the "y" value of a block on the planet. This "y" value is relative to the face the block is on.
    ///
    /// * `noise_generator` Used to generate noise values. Seeded for this world seed.
    /// * `(x, y, z)` Block x/y/z in the structure
    /// * `(structure_x, structure_y, structure_z)` Where the structure is in the universe - used to offset the noise values so no two structures are the same.
    /// * `(middle_air_start)` The midpoint of the extremes of heights. Aka if noise generates 0, then this should return middle_air_start.
    /// * `amplitude` Value passed in by the `GenerationParemeters`. Represents how tall the terrain will be
    /// * `delta` Value passed in by the `GenerationParemeters`. Represents how much each change in x/y/z will effect the terrain. Small values = lesser effect
    /// * `iterations` Value passed in by the `GenerationParemeters`. Represents how many times the noise function will be run
    fn get_block_height(
        noise_generator: &noise::OpenSimplex,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        middle_air_start: CoordinateType,
        amplitude: f64,
        delta: f64,
        iterations: usize,
    ) -> f64 {
        get_block_height(
            noise_generator,
            block_coords,
            structure_coords,
            middle_air_start,
            amplitude,
            delta,
            iterations,
        )
    }

    /// Returns how much the edge height should be averaged in from the other side it's approaching.
    ///
    /// Don't touch this unless you're doing something extremely crazy.
    ///
    /// - `a` x, y, or z but generalized.
    /// - `intersection` is where the two edges are projected to meet, which is used as the limit to your height.
    /// - `s_dimensions` structure width/height/length.
    fn get_mirror_coefficient(a: CoordinateType, intersection: CoordinateType, s_dimensions: CoordinateType) -> f64 {
        let max = intersection;
        let min = intersection - GUIDE_MIN;
        if a > max || a < s_dimensions - max {
            1.0
        } else if a > min {
            1.0 - (max - a) as f64 / (max - min) as f64
        } else if a < s_dimensions - min {
            1.0 - ((a - (s_dimensions - max)) as f64 / (max - min) as f64)
        } else {
            0.0
        }
    }

    /// "Where the math happens" - Dan.
    ///
    /// Combining two linear gradients so that they have the same end behaviors is "a little difficult". Thus the max functions.
    ///
    /// No touchy.
    ///
    /// - `height` If you were at the center of the face of a planet - that's how tall this column would be.
    /// - `c1` The first edge coefficient (from `get_mirror_coefficient`).
    /// - `c1_height` The height on c1's edge.
    /// - `c2` The second edge coefficient (from `get_mirror_coefficient`).
    /// - `c2_height` The height on c2's edge.
    fn merge(height: f64, c1: f64, c1_height: f64, c2: f64, c2_height: f64) -> CoordinateType {
        let c = if c1 + c2 == 0.0 { 0.0 } else { c1.max(c2) / (c1 + c2) };
        (height * (1.0 - c * (c1 + c2)) + c * (c1 * c1_height + c2 * c2_height)) as CoordinateType
    }

    /// Generates the "old" height, the one that's used if you're in the middle of a face.
    /// Also generates the height at any edge within GUIDE_MIN distance.
    /// Averages the "old" height with the edge heights with coefficients based on how close you are to the edge intersection.
    fn guide(
        noise_generator: &noise::OpenSimplex,
        block_up: BlockFace,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        middle_air_start: CoordinateType,
        amplitude: f64,
        delta: f64,
        iterations: usize,
        s_dimensions: CoordinateType,
    ) -> CoordinateType {
        // The amplitude * iterations is an approximation to account for needing to guide the terrain farther from the edge
        // the bumpier the terrain is. Terrain may still get too bumpy.
        let top = middle_air_start - (amplitude * iterations as f64) as CoordinateType;
        let bottom = s_dimensions - top;
        let min = top - GUIDE_MIN;

        // X.
        let mut x_coefficient = 0.0;
        let mut x_height = 0.0;
        if block_coords.x > min || block_coords.x < s_dimensions - min {
            let x_coord = if block_coords.x > s_dimensions / 2 { top } else { bottom };
            let x_seed = match block_up {
                BlockFace::Front => (x_coord, block_coords.y.clamp(bottom, top), top),
                BlockFace::Back => (x_coord, block_coords.y.clamp(bottom, top), bottom),
                BlockFace::Top => (x_coord, top, block_coords.z.clamp(bottom, top)),
                BlockFace::Bottom => (x_coord, bottom, block_coords.z.clamp(bottom, top)),
                BlockFace::Right => (x_coord, block_coords.y, block_coords.z),
                BlockFace::Left => (x_coord, block_coords.y, block_coords.z),
            }
            .into();
            x_height = self::get_block_height(
                noise_generator,
                x_seed,
                structure_coords,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );
            x_coefficient = Self::get_mirror_coefficient(block_coords.x, x_height as CoordinateType, s_dimensions);
        }

        // Y.
        let mut y_coefficient = 0.0;
        let mut y_height = 0.0;
        if block_coords.y > min || block_coords.y < s_dimensions - min {
            let y_coord = if block_coords.y > s_dimensions / 2 { top } else { bottom };
            let y_seed = match block_up {
                BlockFace::Front => (block_coords.x.clamp(bottom, top), y_coord, top),
                BlockFace::Back => (block_coords.x.clamp(bottom, top), y_coord, bottom),
                BlockFace::Top => (block_coords.x, y_coord, block_coords.z),
                BlockFace::Bottom => (block_coords.x, y_coord, block_coords.z),
                BlockFace::Right => (top, y_coord, block_coords.z.clamp(bottom, top)),
                BlockFace::Left => (bottom, y_coord, block_coords.z.clamp(bottom, top)),
            }
            .into();
            y_height = self::get_block_height(
                noise_generator,
                y_seed,
                structure_coords,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );
            y_coefficient = Self::get_mirror_coefficient(block_coords.y, y_height as CoordinateType, s_dimensions);
        }

        // Z.
        let mut z_coefficient = 0.0;
        let mut z_height = 0.0;
        if block_coords.z > min || block_coords.z < s_dimensions - min {
            let z_coord = if block_coords.z > s_dimensions / 2 { top } else { bottom };
            let z_seed = match block_up {
                BlockFace::Front => (block_coords.x, block_coords.y, z_coord),
                BlockFace::Back => (block_coords.x, block_coords.y, z_coord),
                BlockFace::Top => (block_coords.x.clamp(bottom, top), top, z_coord),
                BlockFace::Bottom => (block_coords.x.clamp(bottom, top), bottom, z_coord),
                BlockFace::Right => (top, block_coords.y.clamp(bottom, top), z_coord),
                BlockFace::Left => (bottom, block_coords.y.clamp(bottom, top), z_coord),
            }
            .into();
            z_height = self::get_block_height(
                noise_generator,
                z_seed,
                structure_coords,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );
            z_coefficient = Self::get_mirror_coefficient(block_coords.z, z_height as CoordinateType, s_dimensions);
        }

        match block_up {
            BlockFace::Front | BlockFace::Back => Self::merge(z_height, x_coefficient, x_height, y_coefficient, y_height),
            BlockFace::Top | BlockFace::Bottom => Self::merge(y_height, x_coefficient, x_height, z_coefficient, z_height),
            BlockFace::Right | BlockFace::Left => Self::merge(x_height, y_coefficient, y_height, z_coefficient, z_height),
        }
    }

    /// Gets the top block's height
    ///
    /// * `(x, y, z)` Block x/y/z in the structure
    /// * `(structure_x, structure_y, structure_z)` Where the structure is in the universe - used to offset the noise values so no two structures are the same.
    /// * `(s_dimensions)` The width/height/length of the structure this is on.
    /// * `noise_generator` Used to generate noise values. Seeded for this world seed.
    /// * `(middle_air_start)` The midpoint of the extremes of heights. Aka if noise generates 0, then this should return middle_air_start.
    /// * `amplitude` Value passed in by the `GenerationParemeters`. Represents how tall the terrain will be
    /// * `delta` Value passed in by the `GenerationParemeters`. Represents how much each change in x/y/z will effect the terrain. Small values = lesser effect
    /// * `iterations` Value passed in by the `GenerationParemeters`. Represents how many times the noise function will be run
    fn get_top_height(
        block_up: BlockFace,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &noise::OpenSimplex,
        middle_air_start: CoordinateType,
        amplitude: f64,
        delta: f64,
        iterations: usize,
    ) -> CoordinateType {
        Self::guide(
            noise_generator,
            block_up,
            block_coords,
            structure_coords,
            middle_air_start,
            amplitude,
            delta,
            iterations,
            s_dimensions,
        )
    }
}

/// The default implementation for the `BiosphereGenerationStrategy` that will work for most biospheres.
pub struct DefaultBiosphereGenerationStrategy;

impl BiosphereGenerationStrategy for DefaultBiosphereGenerationStrategy {}

/// Stores which blocks make up each biosphere, and how far below the top solid block each block generates.
/// Blocks in ascending order ("stone" = 5 first, "grass" = 0 last).
#[derive(Resource, Clone, Default, Debug)]
pub struct BlockLayers<T: Component + Clone + Default> {
    _phantom: PhantomData<T>,
    ranges: Vec<(Block, BlockLayer)>,
    sea_block: Option<Block>,
    sea_level: Option<CoordinateType>,
}

/// Stores the blocks and all the noise information for creating the top of their layer.
/// For example, the "stone" BlockLevel has the noise paramters that create the boundry between dirt and stone.
#[derive(Clone, Debug)]
pub struct BlockLayer {
    middle_depth: CoordinateType,
    delta: f64,
    amplitude: f64,
    iterations: usize,
}

impl BlockLayer {
    /// This layer doesn't use a noise function to generate its span, and is thus fixed at a certain depth.
    pub fn fixed_layer(middle_depth: CoordinateType) -> Self {
        Self {
            middle_depth,
            delta: 0.0,
            amplitude: 0.0,
            iterations: 0,
        }
    }

    /// This layer is based off a noise function and will appear at a varying depth based on the parameters
    pub fn noise_layer(middle_depth: CoordinateType, delta: f64, amplitude: f64, iterations: usize) -> Self {
        Self {
            middle_depth,
            delta,
            amplitude,
            iterations,
        }
    }
}

#[derive(Debug)]
/// Errors generated when initally setting up the block ranges
pub enum BlockRangeError<T: Component + Clone + Default> {
    /// This means the block id provided was not found in the block registry
    MissingBlock(BlockLayers<T>),
}

impl<T: Component + Clone + Default> BlockLayers<T> {
    /// Creates a new block range, for each planet type to specify its blocks.
    pub fn new() -> Self {
        Self::default()
    }

    /// Does what `add_fixed_layer` does, but makes the layer depth vary based off the noise parameters.
    pub fn add_noise_layer(
        mut self,
        block_id: &str,
        block_registry: &Registry<Block>,
        middle_depth: CoordinateType,
        delta: f64,
        amplitude: f64,
        iterations: usize,
    ) -> Result<Self, BlockRangeError<T>> {
        let Some(block) = block_registry.from_id(block_id) else {
            return Err(BlockRangeError::MissingBlock(self));
        };
        let layer = BlockLayer::noise_layer(middle_depth, delta, amplitude, iterations);
        self.ranges.push((block.clone(), layer));
        Ok(self)
    }

    /// Use this to construct the various ranges of the blocks.
    ///
    /// The order you add the ranges in DOES matter.
    ///
    /// middle_depth represents how many blocks from the previous layer this block will appear.
    /// For example, If grass was 100, dirt was 1, and stone was 4, it would generate as:
    /// - 100 blocks of air
    /// - Grass
    /// - Dirt
    /// - Dirt
    /// - Dirt
    /// - Dirt
    /// - Stone
    /// - Stone
    /// - Stone
    /// - ... stone down to the bottom
    pub fn add_fixed_layer(
        mut self,
        block_id: &str,
        block_registry: &Registry<Block>,
        middle_depth: CoordinateType,
    ) -> Result<Self, BlockRangeError<T>> {
        let Some(block) = block_registry.from_id(block_id) else {
            return Err(BlockRangeError::MissingBlock(self));
        };
        let layer = BlockLayer::fixed_layer(middle_depth);
        self.ranges.push((block.clone(), layer));
        Ok(self)
    }

    /// Sets the sea level and the block that goes along with it
    pub fn with_sea_level_block(
        mut self,
        block_id: &str,
        block_registry: &Registry<Block>,
        sea_level: CoordinateType,
    ) -> Result<Self, BlockRangeError<T>> {
        let Some(block) = block_registry.from_id(block_id).cloned() else {
            return Err(BlockRangeError::MissingBlock(self));
        };
        self.sea_block = Some(block);
        self.sea_level = Some(sea_level);
        Ok(self)
    }

    #[inline]
    fn sea_block(&self) -> Option<&Block> {
        self.sea_block.as_ref()
    }

    fn face_block<'a>(
        &self,
        height: CoordinateType,
        block_layers: &[(&'a Block, CoordinateType)],
        sea_level: Option<CoordinateType>,
        sea_block: Option<&'a Block>,
        scale: CoordinateType,
    ) -> Option<&'a Block> {
        if scale == 1 {
            for &(block, level_top) in block_layers.iter().rev() {
                if height <= level_top {
                    return Some(block);
                }
            }
        } else {
            let mut itr = block_layers.iter().rev();
            while let Some(&(block, level_top)) = itr.next() {
                if height <= level_top {
                    if height + scale > level_top {
                        let mut last_block = block;

                        while let Some(&(block, level_top)) = itr.next() {
                            last_block = block;

                            if height + scale <= level_top {
                                return Some(block);
                            }
                        }

                        return Some(last_block);
                    }

                    return Some(block);
                }
            }
        }

        // No land blocks, must be sea or air.
        if sea_level.map(|sea_level| height <= sea_level).unwrap_or(false) {
            Some(sea_block.expect("Set sea level without setting a sea block."))
        } else {
            None
        }
    }

    fn edge_block<'a>(
        &self,
        j_height: CoordinateType,
        k_height: CoordinateType,
        j_layers: &[(&'a Block, CoordinateType)],
        k_layers: &[(&'a Block, CoordinateType)],
        sea_level: Option<CoordinateType>,
        sea_block: Option<&'a Block>,
        scale: CoordinateType,
    ) -> Option<&'a Block> {
        if scale == 1 {
            for (index, &(block, j_layer_top)) in j_layers.iter().enumerate().rev() {
                if j_height <= j_layer_top && k_height <= k_layers[index].1 {
                    return Some(block);
                }
            }
        } else {
            let mut itr = j_layers.iter().enumerate().rev();
            while let Some((index, &(block, j_layer_top))) = itr.next() {
                if j_height <= j_layer_top && k_height <= k_layers[index].1 {
                    if j_height + scale > j_layer_top || k_height + scale > k_layers[index].1 {
                        let mut last_block = block;

                        while let Some((index, &(block, j_layer_top))) = itr.next() {
                            last_block = block;

                            if j_height + scale > j_layer_top && k_height + scale > k_layers[index].1 {
                                return Some(block);
                            }
                        }

                        return Some(last_block);
                    }

                    return Some(block);
                }
            }
        }

        // No land blocks, must be sea or air.
        if sea_level.map(|sea_level| j_height.max(k_height) <= sea_level).unwrap_or(false) {
            Some(sea_block.expect("Set sea level without setting a sea block."))
        } else {
            None
        }
    }

    fn corner_block<'a>(
        &self,
        x_height: CoordinateType,
        y_height: CoordinateType,
        z_height: CoordinateType,
        x_layers: &[(&'a Block, CoordinateType)],
        y_layers: &[(&'a Block, CoordinateType)],
        z_layers: &[(&'a Block, CoordinateType)],
        sea_level: Option<CoordinateType>,
        sea_block: Option<&'a Block>,
        scale: CoordinateType,
    ) -> Option<&'a Block> {
        if scale == 1 {
            for (index, &(block, x_layer_top)) in x_layers.iter().enumerate().rev() {
                if x_height <= x_layer_top && y_height <= y_layers[index].1 && z_height <= z_layers[index].1 {
                    return Some(block);
                }
            }
        } else {
            let mut itr = x_layers.iter().enumerate().rev();
            while let Some((index, &(block, x_layer_top))) = itr.next() {
                if x_height <= x_layer_top && y_height <= y_layers[index].1 && z_height <= z_layers[index].1 {
                    if x_height + scale > x_layer_top || y_height + scale > y_layers[index].1 || z_height + scale > z_layers[index].1 {
                        let mut last_block = block;

                        while let Some((index, &(block, x_layer_top))) = itr.next() {
                            last_block = block;

                            if x_height + scale > x_layer_top
                                && y_height + scale > y_layers[index].1
                                && z_height + scale > z_layers[index].1
                            {
                                return Some(block);
                            }
                        }

                        return Some(last_block);
                    }

                    return Some(block);
                }
            }
        }

        // No land blocks, must be sea or air.
        if sea_level
            .map(|sea_level| x_height.max(y_height).max(z_height) <= sea_level)
            .unwrap_or(false)
        {
            Some(sea_block.expect("Set sea level without setting a sea block."))
        } else {
            None
        }
    }
}

fn generate<T: Component + Default + Clone, S: BiosphereGenerationStrategy + 'static>(
    generating_lod: &mut GeneratingLod,
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    first_block_coord: BlockCoordinate,
    s_dimensions: CoordinateType,
    scale: CoordinateType,
    noise_generator: &noise::OpenSimplex,
    block_ranges: &BlockLayers<T>,
) {
    let mut lod_chunk = Box::new(LodChunk::new());

    let chunk_faces = Planet::chunk_planet_faces_with_scale(first_block_coord, s_dimensions, scale);
    match chunk_faces {
        ChunkFaces::Face(up) => {
            generate_face_chunk::<S, T, LodChunk>(
                first_block_coord,
                (structure_x, structure_y, structure_z),
                s_dimensions,
                &noise_generator,
                &block_ranges,
                &mut lod_chunk,
                up,
                scale,
            );
        }
        ChunkFaces::Edge(j_up, k_up) => {
            generate_edge_chunk::<S, T, LodChunk>(
                first_block_coord,
                (structure_x, structure_y, structure_z),
                s_dimensions,
                &noise_generator,
                &block_ranges,
                &mut lod_chunk,
                j_up,
                k_up,
                scale,
            );
        }
        ChunkFaces::Corner(x_up, y_up, z_up) => {
            generate_corner_chunk::<S, T, LodChunk>(
                first_block_coord,
                (structure_x, structure_y, structure_z),
                s_dimensions,
                &noise_generator,
                &block_ranges,
                &mut lod_chunk,
                x_up,
                y_up,
                z_up,
                scale,
            );
        }
    }

    // lod_chunk.fill(blocks.from_id("cosmos:grass").expect("Missing grass!"), BlockFace::Top);
    *generating_lod = GeneratingLod::DoneGenerating(lod_chunk);
}

fn recurse<T: Component + Default + Clone, S: BiosphereGenerationStrategy + 'static>(
    generating_lod: &mut GeneratingLod,
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    first_block_coord: BlockCoordinate,
    s_dimensions: CoordinateType,
    scale: CoordinateType,
    noise_generator: &Noise,
    block_ranges: &BlockLayers<T>,
) {
    match generating_lod {
        GeneratingLod::NeedsGenerated => {
            *generating_lod = GeneratingLod::BeingGenerated;
            generate::<T, S>(
                generating_lod,
                (structure_x, structure_y, structure_z),
                first_block_coord,
                s_dimensions,
                scale,
                noise_generator,
                block_ranges,
            );
        }
        GeneratingLod::Children(children) => {
            let s2 = scale / 2;

            let sc = s2 * CHUNK_DIMENSIONS;

            let coords = [
                (0, 0, 0),
                (0, 0, sc),
                (sc, 0, sc),
                (sc, 0, 0),
                (0, sc, 0),
                (0, sc, sc),
                (sc, sc, sc),
                (sc, sc, 0),
            ];

            children.par_iter_mut().zip(coords).for_each(|(child, (bx, by, bz))| {
                recurse::<T, S>(
                    child,
                    (structure_x, structure_y, structure_z),
                    BlockCoordinate::new(bx, by, bz) + first_block_coord,
                    s_dimensions,
                    s2,
                    noise_generator,
                    block_ranges,
                );
            });
        }
        _ => {}
    }
}

pub(crate) fn begin_generating_lods<T: Component + Default + Clone, S: BiosphereGenerationStrategy + 'static>(
    query: Query<(Entity, &LodNeedsGeneratedForPlayer), With<T>>,
    is_biosphere: Query<(&Structure, &Location), With<T>>,
    noise_generator: Res<ReadOnlyNoise>,
    block_ranges: Res<BlockLayers<T>>,
    mut currently_generating: ResMut<GeneratingLods<T>>,
    mut commands: Commands,
) {
    for (entity, generating_lod) in query.iter() {
        commands.entity(entity).despawn_recursive();

        let Ok((structure, location)) = is_biosphere.get(generating_lod.structure_entity) else {
            return;
        };

        let (player_entity, structure_entity) = (generating_lod.player_entity, generating_lod.structure_entity);

        let task_pool = AsyncComputeTaskPool::get();

        let structure_coords = location.absolute_coords_f64();

        let dimensions = structure.block_dimensions().x;

        let mut generating_lod = generating_lod.clone();
        let noise_generator = noise_generator.clone();
        let block_ranges = block_ranges.clone();

        let task = task_pool.spawn(async move {
            let noise = noise_generator.inner();

            recurse::<T, S>(
                &mut generating_lod.generating_lod,
                (structure_coords.x, structure_coords.y, structure_coords.z),
                BlockCoordinate::new(0, 0, 0),
                dimensions,
                dimensions / CHUNK_DIMENSIONS,
                &noise,
                &block_ranges,
            );

            let lod_delta = recursively_create_lod_delta(generating_lod.generating_lod);
            let cloned_delta = lod_delta.clone();

            let new_lod = if let Some(read_only_current_lod) = generating_lod.current_lod {
                let mut current_lod = read_only_current_lod.inner().clone();
                cloned_delta.apply_changes(&mut current_lod);
                current_lod
            } else {
                cloned_delta.create_lod()
            };

            // lod delta is only used for network requests, so serializing it here saves a ton of processing power on the main thread
            let lod_delta = cosmos_encoder::serialize(&LodNetworkMessage::SetLod(SetLodMessage {
                serialized_lod: cosmos_encoder::serialize(&lod_delta),
                structure: structure_entity,
            }));

            let cloned_new_lod = new_lod.clone();

            DoneGeneratingLod {
                lod_delta,
                new_lod,
                cloned_new_lod,
            }
        });

        println!("Beginning generation of actual lods for {structure_entity:?}");
        currently_generating.push(AsyncGeneratingLod::<T>::new(player_entity, structure_entity, task));
    }
}

fn recursively_create_lod_delta(generated_lod: GeneratingLod) -> LodDelta {
    match generated_lod {
        GeneratingLod::Same => LodDelta::NoChange,
        GeneratingLod::Children(children) => {
            let [c0, c1, c2, c3, c4, c5, c6, c7] = *children;

            LodDelta::Children(Box::new([
                recursively_create_lod_delta(c0),
                recursively_create_lod_delta(c1),
                recursively_create_lod_delta(c2),
                recursively_create_lod_delta(c3),
                recursively_create_lod_delta(c4),
                recursively_create_lod_delta(c5),
                recursively_create_lod_delta(c6),
                recursively_create_lod_delta(c7),
            ]))
        }
        GeneratingLod::DoneGenerating(lod_chunk) => LodDelta::Single(lod_chunk),
        _ => {
            warn!("Invalid lod state: {generated_lod:?}");
            LodDelta::None
        }
    }
}

/// Calls generate_face_chunk, generate_edge_chunk, and generate_corner_chunk to generate the chunks of a planet.
pub fn generate_planet<T: Component + Clone + Default, E: TGenerateChunkEvent + Send + Sync + 'static, S: BiosphereGenerationStrategy>(
    mut query: Query<(&mut Structure, &Location)>,
    mut generating: ResMut<GeneratingChunks<T>>,
    mut events: EventReader<E>,
    noise_generator: Res<ReadOnlyNoise>,
    block_ranges: Res<BlockLayers<T>>,
) {
    let chunks = events
        .iter()
        .filter_map(|ev| {
            let structure_entity = ev.get_structure_entity();
            let coords = ev.get_chunk_coordinates();

            if let Ok((mut structure, _)) = query.get_mut(structure_entity) {
                let Structure::Dynamic(planet) = structure.as_mut() else {
                    panic!("A planet must be dynamic!");
                };
                Some((structure_entity, planet.take_or_create_chunk_for_loading(coords)))
            } else {
                None
            }
        })
        .collect::<Vec<(Entity, Chunk)>>();

    let thread_pool = AsyncComputeTaskPool::get();

    let chunks = chunks
        .into_iter()
        .flat_map(|(structure_entity, chunk)| {
            let Ok((structure, location)) = query.get(structure_entity) else {
                return None;
            };

            let Structure::Dynamic(planet) = structure else {
                panic!("A planet must be dynamic!");
            };

            let s_dimensions = planet.block_dimensions();
            let location = *location;

            Some((chunk, s_dimensions, location, structure_entity))
        })
        .collect::<Vec<(Chunk, CoordinateType, Location, Entity)>>();

    if !chunks.is_empty() {
        for (mut chunk, s_dimensions, location, structure_entity) in chunks {
            let block_ranges = block_ranges.clone();

            let noise = noise_generator.clone();

            let task = thread_pool.spawn(async move {
                let noise_generator = noise.inner();
                // let timer = UtilsTimer::start();

                let actual_pos = location.absolute_coords_f64();

                let structure_z = actual_pos.z;
                let structure_y = actual_pos.y;
                let structure_x = actual_pos.x;

                // To save multiplication operations later.
                let first_block_coord = chunk.chunk_coordinates().first_structure_block();

                // Get all possible planet faces from the chunk corners.
                let chunk_faces = Planet::chunk_planet_faces(first_block_coord, s_dimensions);
                match chunk_faces {
                    ChunkFaces::Face(up) => {
                        generate_face_chunk::<S, T, Chunk>(
                            first_block_coord,
                            (structure_x, structure_y, structure_z),
                            s_dimensions,
                            &noise_generator,
                            &block_ranges,
                            &mut chunk,
                            up,
                            1,
                        );
                    }
                    ChunkFaces::Edge(j_up, k_up) => {
                        generate_edge_chunk::<S, T, Chunk>(
                            first_block_coord,
                            (structure_x, structure_y, structure_z),
                            s_dimensions,
                            &noise_generator,
                            &block_ranges,
                            &mut chunk,
                            j_up,
                            k_up,
                            1,
                        );
                    }
                    ChunkFaces::Corner(x_up, y_up, z_up) => {
                        generate_corner_chunk::<S, T, Chunk>(
                            first_block_coord,
                            (structure_x, structure_y, structure_z),
                            s_dimensions,
                            &noise_generator,
                            &block_ranges,
                            &mut chunk,
                            x_up,
                            y_up,
                            z_up,
                            1,
                        );
                    }
                }
                // timer.log_duration("Chunk:");
                (chunk, structure_entity)
            });

            generating.generating.push(GeneratingChunk::new(task));
        }
    }
}
