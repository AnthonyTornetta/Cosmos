//! Responsible for the default generation of biospheres.

use std::{marker::PhantomData, mem::swap};

use bevy::{
    prelude::{Component, Entity, EventReader, EventWriter, Query, Res, ResMut, Resource},
    tasks::AsyncComputeTaskPool,
};
use cosmos_core::{
    block::{Block, BlockFace},
    physics::location::Location,
    registry::Registry,
    structure::{
        chunk::{Chunk, CHUNK_DIMENSIONS},
        planet::{ChunkFaces, Planet},
        Structure,
    },
    utils::{resource_wrapper::ResourceWrapper, timer::UtilsTimer},
};
use futures_lite::future;
use noise::NoiseFn;

use super::{GeneratingChunk, GeneratingChunks, TGenerateChunkEvent};

/// Some chunks might not be getting flattened, or maybe I'm just crazy.
/// Within (flattening_fraction * planet size) of the 45 starts the flattening.
const FLAT_FRACTION: f64 = 0.4;

/// This fraction of the original depth always remains, even on the very edge of the world.
const UNFLATTENED: f64 = 0.25;

/// Tells the chunk to generate its features.
pub struct GenerateChunkFeaturesEvent<T: Component> {
    _phantom: PhantomData<T>,
    /// cx, cy, cz.
    pub chunk_coords: (usize, usize, usize),
    /// The structure entity that contains this chunk.
    pub structure_entity: Entity,
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
                let (x, y, z) = (chunk.structure_x(), chunk.structure_y(), chunk.structure_z());

                structure.set_chunk(chunk);

                event_writer.send(GenerateChunkFeaturesEvent::<T> {
                    _phantom: PhantomData,
                    structure_entity,
                    chunk_coords: (x, y, z),
                });
            }
        } else {
            generating.generating.push(generating_chunk);
        }
    }
}

#[inline]
fn generate_face_chunk<S: BiosphereGenerationStrategy, T: Component + Clone + Default>(
    (sx, sy, sz): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    s_dimensions: usize,
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    block_ranges: &BlockRanges<T>,
    chunk: &mut Chunk,
    up: BlockFace,
    amplitude: f64,
    delta: f64,
    iterations: usize,
) {
    for i in 0..CHUNK_DIMENSIONS {
        for j in 0..CHUNK_DIMENSIONS {
            let seed_coordinates = match up {
                BlockFace::Top => (sx + i, middle_air_start, sz + j),
                BlockFace::Bottom => (sx + i, s_dimensions - middle_air_start, sz + j),
                BlockFace::Front => (sx + i, sy + j, middle_air_start),
                BlockFace::Back => (sx + i, sy + j, s_dimensions - middle_air_start),
                BlockFace::Right => (middle_air_start, sy + i, sz + j),
                BlockFace::Left => (s_dimensions - middle_air_start, sy + i, sz + j),
            };

            let top_height = S::get_top_height(
                seed_coordinates,
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );

            for height in 0..CHUNK_DIMENSIONS {
                let (x, y, z, actual_height) = match up {
                    BlockFace::Top => (i, height, j, sy + height),
                    BlockFace::Bottom => (i, height, j, s_dimensions - (sy + height)),
                    BlockFace::Front => (i, j, height, sz + height),
                    BlockFace::Back => (i, j, height, s_dimensions - (sz + height)),
                    BlockFace::Right => (height, i, j, sx + height),
                    BlockFace::Left => (height, i, j, s_dimensions - (sx + height)),
                };

                if actual_height <= top_height {
                    let block = block_ranges.face_block(top_height - actual_height);
                    chunk.set_block_at(x, y, z, block, up);
                } else if block_ranges
                    .sea_level
                    .map(|sea_level| actual_height as f32 <= (middle_air_start as f32 + sea_level as f32))
                    .unwrap_or(false)
                {
                    let block = block_ranges.sea_level_block().expect("Sea level set without sea block being set!");
                    chunk.set_block_at(x, y, z, block, up);
                }
            }
        }
    }
}

fn generate_edge_chunk<S: BiosphereGenerationStrategy, T: Component + Clone + Default>(
    (sx, sy, sz): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    s_dimensions: usize,
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    block_ranges: &BlockRanges<T>,
    chunk: &mut Chunk,
    j_up: BlockFace,
    k_up: BlockFace,
    amplitude: f64,
    delta: f64,
    iterations: usize,
) {
    let mut j_top = [[0; CHUNK_DIMENSIONS]; CHUNK_DIMENSIONS];
    for (i, layer) in j_top.iter_mut().enumerate() {
        for (k, height) in layer.iter_mut().enumerate() {
            // Seed coordinates for the noise function. Which loop variable goes to which xyz must agree everywhere.
            let (mut x, mut y, mut z) = (sx + i, sy + i, sz + i);
            match j_up {
                BlockFace::Front => z = middle_air_start,
                BlockFace::Back => z = s_dimensions - middle_air_start,
                BlockFace::Left => x = s_dimensions - middle_air_start,
                BlockFace::Right => x = middle_air_start,
                BlockFace::Top => y = middle_air_start,
                BlockFace::Bottom => y = s_dimensions - middle_air_start,
            };
            match k_up {
                BlockFace::Front | BlockFace::Back => z = sz + k,
                BlockFace::Left | BlockFace::Right => x = sx + k,
                BlockFace::Top | BlockFace::Bottom => y = sy + k,
            };

            // Unmodified top height.
            *height = S::get_top_height(
                (x, y, z),
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );

            // Don't let the top fall "below" the 45.
            let dim_45 = match k_up {
                BlockFace::Front => z,
                BlockFace::Back => s_dimensions - z,
                BlockFace::Left => s_dimensions - x,
                BlockFace::Right => x,
                BlockFace::Top => y,
                BlockFace::Bottom => s_dimensions - y,
            };
            *height = (*height).max(dim_45);
        }
    }

    for i in 0..CHUNK_DIMENSIONS {
        // The minimum (j, j) on the 45 where the two top heights intersect.
        let mut first_both_45 = s_dimensions;
        for j in 0..CHUNK_DIMENSIONS {
            // Seed coordinates for the noise function. Which loop variable goes to which xyz must agree everywhere.
            let (mut x, mut y, mut z) = (sx + i, sy + i, sz + i);
            match k_up {
                BlockFace::Front => z = middle_air_start,
                BlockFace::Back => z = s_dimensions - middle_air_start,
                BlockFace::Left => x = s_dimensions - middle_air_start,
                BlockFace::Right => x = middle_air_start,
                BlockFace::Top => y = middle_air_start,
                BlockFace::Bottom => y = s_dimensions - middle_air_start,
            };
            match j_up {
                BlockFace::Front | BlockFace::Back => z = sz + j,
                BlockFace::Left | BlockFace::Right => x = sx + j,
                BlockFace::Top | BlockFace::Bottom => y = sy + j,
            };

            // Unmodified top height.
            let mut k_top = S::get_top_height(
                (x, y, z),
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );

            // First height, and also the height of the other 45 bc of math.
            let j_height = match j_up {
                BlockFace::Front => z,
                BlockFace::Back => s_dimensions - z,
                BlockFace::Left => s_dimensions - x,
                BlockFace::Right => x,
                BlockFace::Top => y,
                BlockFace::Bottom => s_dimensions - y,
            };

            // Don't let the top height fall "below" the 45, but also don't let it go "above" the first shared 45.
            // This probably won't interfere with anything before the first shared 45 is discovered bc of the loop order.
            k_top = k_top.clamp(j_height, first_both_45);

            // Get smallest top height that's on the 45 for both y and z.
            if j_top[i][j] == j && k_top == j && first_both_45 == s_dimensions {
                first_both_45 = k_top;
            };

            for k in 0..CHUNK_DIMENSIONS {
                // Don't let the top height rise "above" the first shared 45.
                let j_top = j_top[i][k].min(first_both_45);

                // This is super smart I promise, definitely no better way to decide which loop variables are x, y, z.
                let (mut x, mut y, mut z) = (i, i, i);
                match j_up {
                    BlockFace::Front | BlockFace::Back => z = j,
                    BlockFace::Left | BlockFace::Right => x = j,
                    BlockFace::Top | BlockFace::Bottom => y = j,
                };
                match k_up {
                    BlockFace::Front | BlockFace::Back => z = k,
                    BlockFace::Left | BlockFace::Right => x = k,
                    BlockFace::Top | BlockFace::Bottom => y = k,
                };

                // Second height, and also the height of the other 45 (dim_45 in the upper loop must be recalculated here).
                let k_height = match k_up {
                    BlockFace::Front => sz + z,
                    BlockFace::Back => s_dimensions - (sz + z),
                    BlockFace::Left => s_dimensions - (sx + x),
                    BlockFace::Right => sx + x,
                    BlockFace::Top => sy + y,
                    BlockFace::Bottom => s_dimensions - (sy + y),
                };

                // Stops stairways to heaven.
                let num_top: usize = (j_height == j_top) as usize + (k_height == k_top) as usize;
                if j_height <= j_top && k_height <= k_top && num_top <= 1 {
                    // The top block needs different "top" to look good, the block can't tell which "up" looks good.
                    let mut block_up = Planet::get_planet_face_without_structure(sx + x, sy + y, sz + z, s_dimensions);
                    if j_height == j_top {
                        block_up = j_up;
                    }
                    if k_height == k_top {
                        block_up = k_up;
                    }
                    let block = block_ranges.edge_block(j_top - j_height, k_top - k_height);
                    chunk.set_block_at(x, y, z, block, block_up);
                } else if block_ranges
                    .sea_level
                    .map(|sea_level| j_height.max(k_height) as f32 <= (middle_air_start as f32 + sea_level as f32))
                    .unwrap_or(false)
                {
                    let mut block_up = Planet::get_planet_face_without_structure(sx + x, sy + y, sz + z, s_dimensions);
                    if j_height == j_top {
                        block_up = j_up;
                    }
                    if k_height == k_top {
                        block_up = k_up;
                    }

                    let block = block_ranges.sea_level_block().expect("Sea level set without sea block being set!");
                    chunk.set_block_at(x, y, z, block, block_up);
                }
            }
        }
    }
}

fn generate_corner_chunk<S: BiosphereGenerationStrategy, T: Component + Clone + Default>(
    (sx, sy, sz): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    s_dimensions: usize,
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    block_ranges: &BlockRanges<T>,
    chunk: &mut Chunk,
    x_up: BlockFace,
    y_up: BlockFace,
    z_up: BlockFace,
    amplitude: f64,
    delta: f64,
    iterations: usize,
) {
    // x top height cache.
    let mut x_top = [[0; CHUNK_DIMENSIONS]; CHUNK_DIMENSIONS];
    for (j, layer) in x_top.iter_mut().enumerate() {
        for (k, height) in layer.iter_mut().enumerate() {
            // Seed coordinates for the noise function.
            let (x, y, z) = match x_up {
                BlockFace::Right => (middle_air_start, sy + j, sz + k),
                _ => (s_dimensions - middle_air_start, sy + j, sz + k),
            };

            // Unmodified top height.
            *height = S::get_top_height(
                (x, y, z),
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );

            // Don't let the top height fall "below" the 45s.
            let y_45 = match y_up {
                BlockFace::Top => y,
                _ => s_dimensions - y,
            };
            let z_45 = match z_up {
                BlockFace::Front => z,
                _ => s_dimensions - z,
            };
            *height = (*height).max(y_45).max(z_45);
        }
    }

    // y top height cache.
    let mut y_top = [[0; CHUNK_DIMENSIONS]; CHUNK_DIMENSIONS];
    for (i, layer) in y_top.iter_mut().enumerate() {
        for (k, height) in layer.iter_mut().enumerate() {
            // Seed coordinates for the noise function. Which loop variable goes to which xyz must agree everywhere.
            let (x, y, z) = match y_up {
                BlockFace::Top => (sx + i, middle_air_start, sz + k),
                _ => (sx + i, s_dimensions - middle_air_start, sz + k),
            };

            // Unmodified top height.
            *height = S::get_top_height(
                (x, y, z),
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );

            // Don't let the top height fall "below" the 45s.
            let x_45 = match x_up {
                BlockFace::Right => x,
                _ => s_dimensions - x,
            };
            let z_45 = match z_up {
                BlockFace::Front => z,
                _ => s_dimensions - z,
            };
            *height = (*height).max(x_45).max(z_45);
        }
    }

    for i in 0..CHUNK_DIMENSIONS {
        // The minimum (j, j, j) on the 45 where the three top heights intersect.
        let mut first_all_45 = s_dimensions;
        for j in 0..CHUNK_DIMENSIONS {
            // Seed coordinates for the noise function.
            let (x, y, z) = match z_up {
                BlockFace::Front => (sx + i, sy + j, middle_air_start),
                _ => (sx + i, sy + j, s_dimensions - middle_air_start),
            };

            // Unmodified top height.
            let mut z_top = S::get_top_height(
                (x, y, z),
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );

            let x_height = match x_up {
                BlockFace::Right => x,
                _ => s_dimensions - x,
            };

            let y_height = match y_up {
                BlockFace::Top => y,
                _ => s_dimensions - y,
            };

            // Don't let the top height fall "below" the 45, but also don't let it go "above" the first shared 45.
            // This probably won't interfere with anything before the first shared 45 is discovered bc of the loop order.
            z_top = z_top.max(x_height).max(y_height);
            z_top = z_top.min(first_all_45);

            // Get smallest top height that's on the 45 for x, y, and z.
            if x_top[i][j] == j && y_top[i][j] == j && z_top == j && first_all_45 == s_dimensions {
                first_all_45 = z_top;
            };

            for k in 0..CHUNK_DIMENSIONS {
                // Don't let the top rise "above" the first shared 45.
                let x_top = x_top[j][k].min(first_all_45);
                let y_top = y_top[i][k].min(first_all_45);

                let z = sz + k;
                let z_height = match z_up {
                    BlockFace::Front => z,
                    _ => s_dimensions - z,
                };

                // Stops stairways to heaven.
                let num_top: usize = (x_height == x_top) as usize + (y_height == y_top) as usize + (z_height == z_top) as usize;
                if x_height <= x_top && y_height <= y_top && z_height <= z_top && num_top <= 1 {
                    // The top block needs different "top" to look good, the block can't tell which "up" looks good.
                    let mut block_up = Planet::get_planet_face_without_structure(x, y, z, s_dimensions);
                    if x_height == x_top {
                        block_up = x_up;
                    }
                    if y_height == y_top {
                        block_up = y_up;
                    }
                    if z_height == z_top {
                        block_up = z_up;
                    }
                    let block = block_ranges.corner_block(x_top - x_height, y_top - y_height, z_top - z_height);
                    chunk.set_block_at(i, j, k, block, block_up);
                } else if block_ranges
                    .sea_level
                    .map(|sea_level| x_height.max(y_height).max(z_height) as f32 <= (middle_air_start as f32 + sea_level as f32))
                    .unwrap_or(false)
                {
                    let mut block_up = Planet::get_planet_face_without_structure(x, y, z, s_dimensions);
                    if x_height == x_top {
                        block_up = x_up;
                    }
                    if y_height == y_top {
                        block_up = y_up;
                    }
                    if z_height == z_top {
                        block_up = z_up;
                    }

                    let block = block_ranges.sea_level_block().expect("Sea level set without sea block being set!");
                    chunk.set_block_at(x, y, z, block, block_up);
                }
            }
        }
    }
}

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
    fn get_block_depth(
        noise_generator: &noise::OpenSimplex,
        (x, y, z): (usize, usize, usize),
        (structure_x, structure_y, structure_z): (f64, f64, f64),
        middle_air_start: usize,
        amplitude: f64,
        delta: f64,
        iterations: usize,
    ) -> f64 {
        let mut depth: f64 = 0.0;
        for iteration in 1..=iterations {
            let iteration = iteration as f64;
            depth += noise_generator.get([
                (x as f64 + structure_x) * (delta / iteration),
                (y as f64 + structure_y) * (delta / iteration),
                (z as f64 + structure_z) * (delta / iteration),
            ]) * amplitude
                * iteration;
        }

        middle_air_start as f64 + depth
    }

    /// In order to combat artifacts near the edges of planets, this function is called to flatten out the terrain near the corners/edges.
    ///
    /// Unless you're doing something really wacky, you should generally keep this as is.
    ///
    /// * `initial_height` The value returned by `get_block_depth`
    /// * `(x, y, z)` Block x/y/z in the structure
    /// * `(s_dimensions)` The width/height/length of the structure this is on.
    fn flatten(initial_height: f64, middle_air_start: usize, (mut x, mut y, mut z): (usize, usize, usize), s_dimensions: usize) -> usize {
        // For the flattening (it's like the rumbling).
        x = x.min(s_dimensions - x);
        y = y.min(s_dimensions - y);
        z = z.min(s_dimensions - z);

        let mut depth = initial_height - middle_air_start as f64;

        // Min is height of the face you're on, second min is the closer to the 45 of the 2 remaining.
        let dist_from_space = s_dimensions as f64 - initial_height;
        let dist_from_45 = x.min(y).max(x.max(y).min(z)) as f64 - dist_from_space;
        let flattening_limit = (s_dimensions as f64 - 2.0 * dist_from_space) * FLAT_FRACTION;
        depth *= dist_from_45.min(flattening_limit) / flattening_limit * (1.0 - UNFLATTENED) + UNFLATTENED;

        (middle_air_start as f64 + depth).round() as usize
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
        (x, y, z): (usize, usize, usize),
        (structure_x, structure_y, structure_z): (f64, f64, f64),
        s_dimensions: usize,
        noise_generator: &noise::OpenSimplex,
        middle_air_start: usize,
        amplitude: f64,
        delta: f64,
        iterations: usize,
    ) -> usize {
        Self::flatten(
            Self::get_block_depth(
                noise_generator,
                (x, y, z),
                (structure_x, structure_y, structure_z),
                middle_air_start,
                amplitude,
                delta,
                iterations,
            ),
            middle_air_start,
            (x, y, z),
            s_dimensions,
        )
    }
}

/// The default implementation for the `BiosphereGenerationStrategy` that will work for most biospheres.
pub struct DefaultBiosphereGenerationStrategy;

impl BiosphereGenerationStrategy for DefaultBiosphereGenerationStrategy {}

#[derive(Debug, Resource, Clone, Copy)]
/// Stores the information required by the noise-function terrain generation to create your terrain.
pub struct GenerationParemeters<T: Component + Clone + Default> {
    /// How big of a difference each x/y/z coordinate makes. Higher values
    /// procude more jagged-looking terrain.
    pub delta: f64,
    /// How many times the noise function will be applied. 9 is generally a good number,
    /// but experiment. Higher values will result in higher/lower extremes.
    pub iterations: usize,
    /// This determines how high/low the terrain can generate. If `iterations` != 1 then
    /// this does not exactly correlate to how tall the terrain will be.
    pub amplitude: f64,
    _phantom: PhantomData<T>,
}

impl<T: Component + Clone + Default> GenerationParemeters<T> {
    /// Stores the information required by the noise-function terrain generation to create your terrain.
    /// * `delta`
    /// How big of a difference each x/y/z coordinate makes. Higher values
    /// procude more jagged-looking terrain.
    /// * `amplitude`
    /// This determines how high/low the terrain can generate. If `iterations` != 1 then
    /// this does not exactly correlate to how tall the terrain will be.
    /// * `iterations`
    /// How many times the noise function will be applied. 9 is generally a good number,
    /// but experiment. Higher values will result in higher/lower extremes.

    pub fn new(delta: f64, amplitude: f64, iterations: usize) -> Self {
        Self {
            _phantom: PhantomData::default(),
            delta,
            amplitude,
            iterations,
        }
    }
}

/// Stores which blocks make up each biosphere, and how far below the top solid block each block generates.
/// Blocks in ascending order ("stone" = 5 first, "grass" = 0 last).
#[derive(Resource, Clone, Default, Debug)]
pub struct BlockRanges<T: Component + Clone + Default> {
    _phantom: PhantomData<T>,
    ranges: Vec<(Block, usize)>,
    sea_level_block: Option<Block>,
    sea_level: Option<i32>,
}

#[derive(Debug)]
/// Errors generated when initally setting up the block ranges
pub enum BlockRangeError<T: Component + Clone + Default> {
    /// This means the block id provided was not found in the block registry
    MissingBlock(BlockRanges<T>),
}

impl<T: Component + Clone + Default> BlockRanges<T> {
    /// Creates a new block range, for each planet type to specify its blocks.
    pub fn new() -> Self {
        Self::default()
    }

    /// Use this to construct the various ranges of the blocks.
    ///
    /// The order you add the ranges in does not matter.
    ///
    /// n_blocks_from_top represents how many blocks down this block will appear.
    /// For example, If grass was 0, dirt was 1, and stone was 5, it would generate as:
    ///
    /// - Grass
    /// - Dirt
    /// - Dirt
    /// - Dirt
    /// - Dirt
    /// - Stone
    /// - Stone
    /// - Stone
    /// - ... stone down to the bottom
    pub fn with_range(
        mut self,
        block_id: &str,
        block_registry: &Registry<Block>,
        n_blocks_from_top: usize,
    ) -> Result<Self, BlockRangeError<T>> {
        if let Some(block) = block_registry.from_id(block_id) {
            let first_smaller_idx = self
                .ranges
                .iter()
                .enumerate()
                .find(|(_, (_, other_n_from_top))| *other_n_from_top < n_blocks_from_top)
                .map(|x| x.0);

            let new_val = (block.clone(), n_blocks_from_top);

            if let Some(first_smaller_idx) = first_smaller_idx {
                self.ranges.insert(first_smaller_idx, new_val);
            } else {
                self.ranges.push(new_val);
            }

            Ok(self)
        } else {
            Err(BlockRangeError::MissingBlock(self))
        }
    }

    /// Sets the sea level and the block that goes along with it
    pub fn with_sea_level_block(
        mut self,
        block_id: &str,
        block_registry: &Registry<Block>,
        sea_level: i32,
    ) -> Result<Self, BlockRangeError<T>> {
        if let Some(block) = block_registry.from_id(block_id).cloned() {
            self.sea_level_block = Some(block);
            self.sea_level = Some(sea_level);

            Ok(self)
        } else {
            Err(BlockRangeError::MissingBlock(self))
        }
    }

    #[inline]
    fn sea_level_block(&self) -> Option<&Block> {
        self.sea_level_block.as_ref()
    }

    fn face_block(&self, depth: usize) -> &Block {
        for (block, d) in self.ranges.iter() {
            if depth >= *d {
                return block;
            }
        }
        panic!("No matching block range for depth {depth}.");
    }

    fn edge_block(&self, j_depth: usize, k_depth: usize) -> &Block {
        for (block, d) in self.ranges.iter() {
            if j_depth >= *d && k_depth >= *d {
                return block;
            }
        }
        panic!("No matching block range for depths {j_depth} and {k_depth}.");
    }

    fn corner_block(&self, x_depth: usize, y_depth: usize, z_depth: usize) -> &Block {
        for (block, d) in self.ranges.iter() {
            if x_depth >= *d && y_depth >= *d && z_depth >= *d {
                return block;
            }
        }
        panic!("No matching block range for depths {x_depth}, {y_depth}, and {z_depth}.");
    }
}

/// Calls generate_face_chunk, generate_edge_chunk, and generate_corner_chunk to generate the chunks of a planet.
pub fn generate_planet<T: Component + Clone + Default, E: TGenerateChunkEvent + Send + Sync + 'static, S: BiosphereGenerationStrategy>(
    mut query: Query<(&mut Structure, &Location)>,
    mut generating: ResMut<GeneratingChunks<T>>,
    mut events: EventReader<E>,
    noise_generator: Res<ResourceWrapper<noise::OpenSimplex>>,
    block_ranges: Res<BlockRanges<T>>,
    generation_parameters: Res<GenerationParemeters<T>>,
) {
    let chunks = events
        .iter()
        .filter_map(|ev| {
            let structure_entity = ev.get_structure_entity();
            let (x, y, z) = ev.get_chunk_coordinates();
            if let Ok((mut structure, _)) = query.get_mut(structure_entity) {
                Some((structure_entity, structure.take_or_create_chunk_for_loading(x, y, z)))
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

            let s_dimensions = structure.blocks_length();
            let location = *location;

            Some((chunk, s_dimensions, location, structure_entity))
        })
        .collect::<Vec<(Chunk, usize, Location, Entity)>>();

    if !chunks.is_empty() {
        println!("Doing {} chunks!", chunks.len());

        for (mut chunk, s_dimensions, location, structure_entity) in chunks {
            let block_ranges = block_ranges.clone();
            let noise_generator = **noise_generator;
            let generation_parameters = generation_parameters.clone();

            let task = thread_pool.spawn(async move {
                let timer = UtilsTimer::start();

                let middle_air_start = s_dimensions - CHUNK_DIMENSIONS * 5;

                let actual_pos = location.absolute_coords_f64();

                let structure_z = actual_pos.z;
                let structure_y = actual_pos.y;
                let structure_x = actual_pos.x;

                // To save multiplication operations later.
                let sz = chunk.structure_z() * CHUNK_DIMENSIONS;
                let sy = chunk.structure_y() * CHUNK_DIMENSIONS;
                let sx = chunk.structure_x() * CHUNK_DIMENSIONS;

                // Get all possible planet faces from the chunk corners.
                let chunk_faces = Planet::chunk_planet_faces((sx, sy, sz), s_dimensions);
                match chunk_faces {
                    ChunkFaces::Face(up) => {
                        generate_face_chunk::<S, T>(
                            (sx, sy, sz),
                            (structure_x, structure_y, structure_z),
                            s_dimensions,
                            &noise_generator,
                            middle_air_start,
                            &block_ranges,
                            &mut chunk,
                            up,
                            generation_parameters.amplitude,
                            generation_parameters.delta,
                            generation_parameters.iterations,
                        );
                    }
                    ChunkFaces::Edge(j_up, k_up) => {
                        generate_edge_chunk::<S, T>(
                            (sx, sy, sz),
                            (structure_x, structure_y, structure_z),
                            s_dimensions,
                            &noise_generator,
                            middle_air_start,
                            &block_ranges,
                            &mut chunk,
                            j_up,
                            k_up,
                            generation_parameters.amplitude,
                            generation_parameters.delta,
                            generation_parameters.iterations,
                        );
                    }
                    ChunkFaces::Corner(x_up, y_up, z_up) => {
                        generate_corner_chunk::<S, T>(
                            (sx, sy, sz),
                            (structure_x, structure_y, structure_z),
                            s_dimensions,
                            &noise_generator,
                            middle_air_start,
                            &block_ranges,
                            &mut chunk,
                            x_up,
                            y_up,
                            z_up,
                            generation_parameters.amplitude,
                            generation_parameters.delta,
                            generation_parameters.iterations,
                        );
                    }
                }
                timer.log_duration("Chunk: ");
                (chunk, structure_entity)
            });

            generating.generating.push(GeneratingChunk::new(task));
        }
    }
}
