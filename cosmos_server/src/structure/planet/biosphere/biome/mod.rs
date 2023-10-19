use std::{
    any::Any,
    hash::Hash,
    marker::PhantomData,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use bevy::prelude::{info, App, EventWriter, OnEnter, OnExit, Res, ResMut, Resource, Vec3};
use cosmos_core::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::Registry,
    structure::{
        block_storage::BlockStorer,
        chunk::{Chunk, CHUNK_DIMENSIONS},
        coordinates::{BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate, CoordinateType},
        lod_chunk::LodChunk,
        planet::Planet,
        Structure,
    },
    utils::array_utils::{flatten, flatten_2d},
};
use noise::NoiseFn;

use crate::{init::init_world::Noise, state::GameState};

use self::biome_registry::RegisteredBiome;

use super::{biosphere_generation::BlockLayers, BiosphereMarkerComponent};

pub mod biome_registry;
pub mod plains;

const GUIDE_MIN: CoordinateType = 100;

pub struct SimpleBiome {
    id: u16,
    unlocalized_name: String,
    block_layers: BlockLayers,
}

impl SimpleBiome {
    pub fn new(name: impl Into<String>, block_layers: BlockLayers) -> Self {
        Self {
            id: 0,
            block_layers,
            unlocalized_name: name.into(),
        }
    }
}

impl Biome for SimpleBiome {
    fn block_layers(&self) -> &BlockLayers {
        &self.block_layers
    }

    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }

    fn generate_chunk_features(
        &self,
        block_event_writer: &mut EventWriter<BlockChangedEvent>,
        chunk_coords: ChunkCoordinate,
        structure: &mut Structure,
        structure_location: &Location,
        blocks: &Registry<Block>,
        noise_generator: &Noise,
    ) {
    }
}

#[inline]
fn generate_face_chunk<C: BlockStorer>(
    biome: &dyn Biome,
    block_coords: BlockCoordinate,
    structure_coords: (f64, f64, f64),
    s_dimensions: CoordinateType,
    noise_generator: &noise::OpenSimplex,
    chunk: &mut C,
    up: BlockFace,
    scale: CoordinateType,
    start: ChunkBlockCoordinate,
    stop: ChunkBlockCoordinate,
) {
    let (sx, sy, sz) = (block_coords.x, block_coords.y, block_coords.z);

    let block_layers = biome.block_layers();

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
            for (block, level) in block_layers.ranges() {
                let level_top = biome.get_top_height(
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

                let block = block_layers.face_block(height, &concrete_ranges, block_layers.sea_level(), scale);
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

fn generate_edge_chunk<C: BlockStorer>(
    biome: &dyn Biome,
    block_coords: BlockCoordinate,
    structure_coords: (f64, f64, f64),
    s_dimensions: CoordinateType,
    noise_generator: &noise::OpenSimplex,
    chunk: &mut C,
    j_up: BlockFace,
    k_up: BlockFace,
    scale: CoordinateType,
    start: ChunkBlockCoordinate,
    stop: ChunkBlockCoordinate,
) {
    let block_layers = biome.block_layers();

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
            for (block, layer) in block_layers.ranges() {
                let layer_top = biome.get_top_height(
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
            for (block, layer) in block_layers.ranges() {
                let layer_top = biome.get_top_height(
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
                    let block = block_layers.edge_block(j_height, k_height, j_layers, &k_layers, block_layers.sea_level(), scale);
                    if let Some(block) = block {
                        chunk.set_block_at(chunk_block_coords, block, block_up);
                    }
                }
            }
        }
    }
}

// Might trim 45s, see generate_edge_chunk.
fn generate_corner_chunk<C: BlockStorer>(
    biome: &dyn Biome,
    block_coords: BlockCoordinate,
    structure_coords: (f64, f64, f64),
    s_dimensions: CoordinateType,
    noise_generator: &noise::OpenSimplex,
    chunk: &mut C,
    x_up: BlockFace,
    y_up: BlockFace,
    z_up: BlockFace,
    scale: CoordinateType,
    start: ChunkBlockCoordinate,
    stop: ChunkBlockCoordinate,
) {
    let block_layers = biome.block_layers();

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
            for (block, level) in block_layers.ranges() {
                let level_top = biome.get_top_height(
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
            for (block, level) in block_layers.ranges() {
                let level_top = biome.get_top_height(
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
            for (block, level) in block_layers.ranges() {
                let level_top = biome.get_top_height(
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
                let block = block_layers.corner_block(
                    x_height,
                    y_height,
                    z_height,
                    &x_layers[flatten_2d(j as usize, k as usize, CHUNK_DIMENSIONS as usize)],
                    &y_layers[flatten_2d(i as usize, k as usize, CHUNK_DIMENSIONS as usize)],
                    &z_layers,
                    block_layers.sea_level(),
                    scale,
                );
                if let Some(block) = block {
                    chunk.set_block_at(ChunkBlockCoordinate::new(i, j, k), block, block_up);
                }
            }
        }
    }
}

pub trait Biome: Send + Sync + 'static {
    fn id(&self) -> u16;
    fn unlocalized_name(&self) -> &str;
    fn set_numeric_id(&mut self, id: u16);

    fn block_layers(&self) -> &BlockLayers;

    fn generate_face_chunk_lod(
        &self,
        self_as_dyn: &dyn Biome,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &noise::OpenSimplex,
        chunk: &mut LodChunk,
        up: BlockFace,
        scale: CoordinateType,
        start: ChunkBlockCoordinate,
        stop: ChunkBlockCoordinate,
    ) {
        generate_face_chunk::<LodChunk>(
            self_as_dyn,
            block_coords,
            structure_coords,
            s_dimensions,
            noise_generator,
            chunk,
            up,
            scale,
            start,
            stop,
        );
    }

    fn generate_edge_chunk_lod(
        &self,
        self_as_dyn: &dyn Biome,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &noise::OpenSimplex,
        chunk: &mut LodChunk,
        j_up: BlockFace,
        k_up: BlockFace,
        scale: CoordinateType,
        start: ChunkBlockCoordinate,
        stop: ChunkBlockCoordinate,
    ) {
        generate_edge_chunk(
            self_as_dyn,
            block_coords,
            structure_coords,
            s_dimensions,
            noise_generator,
            chunk,
            j_up,
            k_up,
            scale,
            start,
            stop,
        );
    }

    fn generate_corner_chunk_lod(
        &self,
        self_as_dyn: &dyn Biome,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &noise::OpenSimplex,
        chunk: &mut LodChunk,
        x_up: BlockFace,
        y_up: BlockFace,
        z_up: BlockFace,
        scale: CoordinateType,
        start: ChunkBlockCoordinate,
        stop: ChunkBlockCoordinate,
    ) {
        generate_corner_chunk::<LodChunk>(
            self_as_dyn,
            block_coords,
            structure_coords,
            s_dimensions,
            noise_generator,
            chunk,
            x_up,
            y_up,
            z_up,
            scale,
            start,
            stop,
        );
    }

    fn generate_face_chunk(
        &self,
        self_as_dyn: &dyn Biome,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &noise::OpenSimplex,
        chunk: &mut Chunk,
        up: BlockFace,
        scale: CoordinateType,
        start: ChunkBlockCoordinate,
        stop: ChunkBlockCoordinate,
    ) {
        generate_face_chunk::<Chunk>(
            self_as_dyn,
            block_coords,
            structure_coords,
            s_dimensions,
            noise_generator,
            chunk,
            up,
            scale,
            start,
            stop,
        );
    }

    fn generate_edge_chunk(
        &self,
        self_as_dyn: &dyn Biome,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &noise::OpenSimplex,
        chunk: &mut Chunk,
        j_up: BlockFace,
        k_up: BlockFace,
        scale: CoordinateType,
        start: ChunkBlockCoordinate,
        stop: ChunkBlockCoordinate,
    ) {
        generate_edge_chunk::<Chunk>(
            self_as_dyn,
            block_coords,
            structure_coords,
            s_dimensions,
            noise_generator,
            chunk,
            j_up,
            k_up,
            scale,
            start,
            stop,
        );
    }

    fn generate_corner_chunk(
        &self,
        self_as_dyn: &dyn Biome,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &noise::OpenSimplex,
        chunk: &mut Chunk,
        x_up: BlockFace,
        y_up: BlockFace,
        z_up: BlockFace,
        scale: CoordinateType,
        start: ChunkBlockCoordinate,
        stop: ChunkBlockCoordinate,
    ) {
        generate_corner_chunk::<Chunk>(
            self_as_dyn,
            block_coords,
            structure_coords,
            s_dimensions,
            noise_generator,
            chunk,
            x_up,
            y_up,
            z_up,
            scale,
            start,
            stop,
        );
    }

    fn generate_chunk_features(
        &self,
        block_event_writer: &mut EventWriter<BlockChangedEvent>,
        chunk_coords: ChunkCoordinate,
        structure: &mut Structure,
        structure_location: &Location,
        blocks: &Registry<Block>,
        noise_generator: &Noise,
    );

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
        &self,
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

    /// Returns how much the edge height should be averaged in from the other side it's approaching.
    ///
    /// Don't touch this unless you're doing something extremely crazy.
    ///
    /// - `a` x, y, or z but generalized.
    /// - `intersection` is where the two edges are projected to meet, which is used as the limit to your height.
    /// - `s_dimensions` structure width/height/length.
    fn get_mirror_coefficient(&self, a: CoordinateType, intersection: CoordinateType, s_dimensions: CoordinateType) -> f64 {
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
    fn merge(&self, height: f64, c1: f64, c1_height: f64, c2: f64, c2_height: f64) -> CoordinateType {
        let c = if c1 + c2 == 0.0 { 0.0 } else { c1.max(c2) / (c1 + c2) };
        (height * (1.0 - c * (c1 + c2)) + c * (c1 * c1_height + c2 * c2_height)) as CoordinateType
    }

    /// Generates the "old" height, the one that's used if you're in the middle of a face.
    /// Also generates the height at any edge within GUIDE_MIN distance.
    /// Averages the "old" height with the edge heights with coefficients based on how close you are to the edge intersection.
    fn guide(
        &self,
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
            x_height = self.get_block_height(
                noise_generator,
                x_seed,
                structure_coords,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );
            x_coefficient = self.get_mirror_coefficient(block_coords.x, x_height as CoordinateType, s_dimensions);
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
            y_height = self.get_block_height(
                noise_generator,
                y_seed,
                structure_coords,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );
            y_coefficient = self.get_mirror_coefficient(block_coords.y, y_height as CoordinateType, s_dimensions);
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
            z_height = self.get_block_height(
                noise_generator,
                z_seed,
                structure_coords,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );
            z_coefficient = self.get_mirror_coefficient(block_coords.z, z_height as CoordinateType, s_dimensions);
        }

        match block_up {
            BlockFace::Front | BlockFace::Back => self.merge(z_height, x_coefficient, x_height, y_coefficient, y_height),
            BlockFace::Top | BlockFace::Bottom => self.merge(y_height, x_coefficient, x_height, z_coefficient, z_height),
            BlockFace::Right | BlockFace::Left => self.merge(x_height, y_coefficient, y_height, z_coefficient, z_height),
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
        &self,
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
        self.guide(
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

impl PartialEq for dyn Biome {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for dyn Biome {}

impl Hash for dyn Biome {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u16(self.id())
    }
}

const LOOKUP_TABLE_PRECISION: usize = 100;
const LOOKUP_TABLE_SIZE: usize = LOOKUP_TABLE_PRECISION * LOOKUP_TABLE_PRECISION * LOOKUP_TABLE_PRECISION;

#[derive(Resource, Clone)]
pub struct BiosphereBiomesRegistry<T> {
    _phantom: PhantomData<T>,

    /// Contains a list of indicies to the biomes vec
    lookup_table: Arc<RwLock<[u8; LOOKUP_TABLE_SIZE]>>,

    /// All the registered biomes
    biomes: Vec<Arc<RwLock<Box<dyn Biome>>>>,
    /// Only used before `construct_lookup_table` method is called, used to store the biomes + their [`BiomeParameters`] before all the possibilities are computed.
    todo_biomes: Vec<(Vec3, usize)>,
}

#[derive(Clone, Copy, Debug)]
pub struct BiomeParameters {
    /// This must be within 0.0 to 100.0
    pub ideal_temperature: f32,
    /// This must be within 0.0 to 100.0
    pub ideal_elevation: f32,
    /// This must be within 0.0 to 100.0
    pub ideal_humidity: f32,
}

impl<T> Default for BiosphereBiomesRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BiosphereBiomesRegistry<T> {
    pub fn new() -> Self {
        Self {
            _phantom: Default::default(),
            lookup_table: Arc::new(RwLock::new([0; LOOKUP_TABLE_SIZE])),
            biomes: vec![],
            todo_biomes: Default::default(),
        }
    }

    fn construct_lookup_table(&mut self) {
        info!("Creating biome lookup table! This could take a bit...");

        let mut lookup_table: std::sync::RwLockWriteGuard<'_, [u8; 1000000]> = self.lookup_table.write().unwrap();

        for z in 0..LOOKUP_TABLE_PRECISION {
            for y in 0..LOOKUP_TABLE_PRECISION {
                for x in 0..LOOKUP_TABLE_PRECISION {
                    let mut best_biome: Option<(f32, usize)> = None;

                    let pos = Vec3::new(x as f32, y as f32, z as f32);

                    for &(params, idx) in self.todo_biomes.iter() {
                        if let Some(best_b) = best_biome {
                            let dist = pos.distance_squared(params);
                            if dist < best_b.0 {
                                best_biome = Some((dist, idx));
                            }
                        }
                    }

                    let Some(best_biome) = best_biome else {
                        panic!("Biome registry has no biomes - every biosphere must have at least one biome attached!");
                    };

                    lookup_table[flatten(x, y, z, LOOKUP_TABLE_PRECISION, LOOKUP_TABLE_PRECISION)] = best_biome.1 as u8;
                }
            }
        }

        info!("Done constructing lookup table!");
    }

    pub fn register(&mut self, biome: Arc<RwLock<Box<dyn Biome>>>, params: BiomeParameters) {
        let idx = self.biomes.len();
        self.biomes.push(biome);
        self.todo_biomes.push((
            Vec3::new(params.ideal_temperature, params.ideal_humidity, params.ideal_elevation),
            idx,
        ));
    }

    /// Gets the ideal biome for the parmaters provided
    ///
    /// # Panics
    /// If the params values are outside the range of `[0.0, 100)`, if there was an error getting the RwLock, or if [`construct_lookup_table`] wasn't called yet (run before [`GameState::PostLoading`]` ends)
    pub fn ideal_biome_for(&self, params: BiomeParameters) -> RwLockReadGuard<Box<dyn Biome>> {
        debug_assert!(params.ideal_elevation >= 0.0 && params.ideal_elevation < 100.0);
        debug_assert!(params.ideal_humidity >= 0.0 && params.ideal_humidity < 100.0);
        debug_assert!(params.ideal_temperature >= 0.0 && params.ideal_temperature < 100.0);

        let lookup_idx = flatten(
            params.ideal_elevation as usize,
            params.ideal_humidity as usize,
            params.ideal_temperature as usize,
            LOOKUP_TABLE_PRECISION,
            LOOKUP_TABLE_PRECISION,
        );

        self.biomes[self.lookup_table.read().unwrap()[lookup_idx] as usize].read().unwrap()
    }
}

fn register_biome(mut registry: ResMut<Registry<RegisteredBiome>>, block_registry: Res<Registry<Block>>) {
    registry.register(RegisteredBiome::new(Box::new(SimpleBiome::new(
        "cosmos:plains",
        BlockLayers::default()
            .add_noise_layer("cosmos:grass", &block_registry, 160, 0.05, 7.0, 9)
            .expect("Grass missing")
            .add_fixed_layer("cosmos:dirt", &block_registry, 1)
            .expect("Dirt missing")
            .add_fixed_layer("cosmos:stone", &block_registry, 4)
            .expect("Stone missing"),
    ))));
}

fn construct_lookup_tables<T: BiosphereMarkerComponent>(mut registry: ResMut<BiosphereBiomesRegistry<T>>) {
    registry.construct_lookup_table();
}

pub fn create_biosphere_biomes_registry<T: BiosphereMarkerComponent>(app: &mut App) {
    app.init_resource::<BiosphereBiomesRegistry<T>>()
        .add_systems(OnExit(GameState::PostLoading), construct_lookup_tables::<T>);
}

pub(super) fn register(app: &mut App) {
    biome_registry::register(app);

    app.add_systems(OnEnter(GameState::Loading), register_biome);
}
