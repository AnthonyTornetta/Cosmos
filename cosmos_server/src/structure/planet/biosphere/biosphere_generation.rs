//! Responsible for the default generation of biospheres.

use std::{marker::PhantomData, mem::swap, sync::RwLockReadGuard};

use bevy::{
    log::warn,
    prelude::{Commands, DespawnRecursiveExt, Entity, Event, EventReader, EventWriter, Query, Res, ResMut, Resource, With},
    tasks::AsyncComputeTaskPool,
};
use cosmos_core::{
    block::{Block, BlockFace},
    netty::cosmos_encoder,
    physics::location::Location,
    registry::Registry,
    structure::{
        chunk::{Chunk, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS_USIZE},
        coordinates::{BlockCoordinate, ChunkCoordinate, CoordinateType},
        lod::{LodDelta, LodNetworkMessage, SetLodMessage},
        lod_chunk::LodChunk,
        planet::{ChunkFaces, Planet},
        Structure,
    },
    utils::array_utils::{flatten, flatten_2d},
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

use super::{
    biome::{Biome, BiomeIdList, BiomeParameters, BiosphereBiomesRegistry},
    BiomeDecider, BiosphereMarkerComponent, BiosphereSeaLevel, GeneratingChunk, GeneratingChunks, TGenerateChunkEvent,
};

/// Tells the chunk to generate its features.
#[derive(Debug, Event)]
pub struct GenerateChunkFeaturesEvent<T: BiosphereMarkerComponent> {
    _phantom: PhantomData<T>,
    /// cx, cy, cz.
    pub chunk_coords: ChunkCoordinate,
    /// The structure entity that contains this chunk.
    pub structure_entity: Entity,
}

/// Sends a ChunkInitEvent for every chunk that's done generating, monitors when chunks are finished generating.
pub fn notify_when_done_generating_terrain<T: BiosphereMarkerComponent>(
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

/// Stores which blocks make up each biosphere, and how far below the top solid block each block generates.
/// Blocks in ascending order ("stone" = 5 first, "grass" = 0 last).
#[derive(Resource, Clone, Default, Debug)]
pub struct BlockLayers {
    ranges: Vec<(Block, BlockLayer)>,
}

impl BlockLayers {
    /// Returns an iterator over all the block ranges in the order they were added
    pub fn ranges(&self) -> std::slice::Iter<(cosmos_core::block::Block, BlockLayer)> {
        self.ranges.iter()
    }
}

/// Stores the blocks and all the noise information for creating the top of their layer.
/// For example, the "stone" BlockLevel has the noise paramters that create the boundry between dirt and stone.
#[derive(Clone, Debug)]
pub struct BlockLayer {
    /// How far away from this elevation should this generate
    ///
    /// For the first block, this should almost always be 0.
    ///
    /// For example:
    /// - `Grass` 0
    /// - `Dirt` 1
    /// - `Stone` 4
    ///
    /// Would create 1 top layer of grass starting at the proper elevation, 4 layers of dirt below that, and however many layers of stone till the bottom
    pub middle_depth: CoordinateType,
    /// How much each change in coordinate will effect the change of the block
    ///
    /// Lower number = less change per block.
    pub delta: f64,
    /// Maximum/minimum height of this layer.
    pub amplitude: f64,
    /// # of iterations for this layer. More = more computationally expensive but better looking terrain.
    ///
    /// I would recommend putting iterations to something like 9 for top-level terrain, and keeping it 1 for everything else.
    pub iterations: usize,
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
pub enum BlockRangeError {
    /// This means the block id provided was not found in the block registry
    MissingBlock(BlockLayers),
}

impl BlockLayers {
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
    ) -> Result<Self, BlockRangeError> {
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
    ) -> Result<Self, BlockRangeError> {
        let Some(block) = block_registry.from_id(block_id) else {
            return Err(BlockRangeError::MissingBlock(self));
        };
        let layer = BlockLayer::fixed_layer(middle_depth);
        self.ranges.push((block.clone(), layer));
        Ok(self)
    }

    /// Calculates the block here for a face chunk
    pub fn face_block<'a>(
        &self,
        height: CoordinateType,
        block_layers: &[(&'a Block, CoordinateType)],
        sea_level: Option<(CoordinateType, Option<&'a Block>)>,
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

                        for &(block, level_top) in itr.by_ref() {
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
        if let Some((sea_level, Some(sea_block))) = sea_level {
            if height <= sea_level {
                Some(sea_block)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Calculates the block here for an edge chunk
    pub fn edge_block<'a>(
        &self,
        j_height: CoordinateType,
        k_height: CoordinateType,
        j_layers: &[(&'a Block, CoordinateType)],
        k_layers: &[(&'a Block, CoordinateType)],
        sea_level: Option<(CoordinateType, Option<&'a Block>)>,
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

                        for (index, &(block, j_layer_top)) in itr.by_ref() {
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
        if let Some((sea_level, Some(sea_block))) = sea_level {
            if j_height.max(k_height) <= sea_level {
                Some(sea_block)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Calculates the block here for a corner chunk
    pub fn corner_block<'a>(
        &self,
        x_height: CoordinateType,
        y_height: CoordinateType,
        z_height: CoordinateType,
        x_layers: &[(&'a Block, CoordinateType)],
        y_layers: &[(&'a Block, CoordinateType)],
        z_layers: &[(&'a Block, CoordinateType)],
        sea_level: Option<(CoordinateType, Option<&'a Block>)>,
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

                        for (index, &(block, x_layer_top)) in itr.by_ref() {
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

        if let Some((sea_level, Some(sea_block))) = sea_level {
            if x_height.max(y_height).max(z_height) <= sea_level {
                Some(sea_block)
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn generate_height<T: BiosphereMarkerComponent>(
    biome_decider: &BiomeDecider<T>,
    structure_location: &Location,
    seed_coords: BlockCoordinate,
    noise_generator: &Noise,
    up: BlockFace,
    s_dimensions: CoordinateType,
    sea_level: CoordinateType,
) -> (BiomeParameters, CoordinateType) {
    let mut biome_params = biome_decider.biome_parameters_at(structure_location, seed_coords, noise_generator);

    let s_coords = structure_location.absolute_coords_f64();

    let amplitude = 30;

    let top_height = get_top_height(
        up,
        seed_coords,
        (s_coords.x, s_coords.y, s_coords.z),
        s_dimensions,
        noise_generator,
        sea_level,
        amplitude as f64,
        0.01,
        9,
    );

    let x = 50;

    //   sea_level-x = 0%      sea_level = 50%   sea_level+x = 100%

    let percentage = (top_height.clamp(sea_level - x, sea_level + x) - (sea_level - x)) as f32 / (x as f32 * 2.0) * 100.0;

    biome_params.ideal_elevation = percentage;

    (biome_params, top_height)
}

fn calculate_biomes_and_elevations_face<'a, T: BiosphereMarkerComponent>(
    first_block_coord: BlockCoordinate,
    structure_location: &Location,
    noise_generator: &Noise,
    scale: CoordinateType,
    biome_decider: &BiomeDecider<T>,
    biosphere_biomes: &'a BiosphereBiomesRegistry<T>,
    sea_level: CoordinateType,
    up: BlockFace,
    s_dimensions: CoordinateType,
) -> (
    Vec<(RwLockReadGuard<'a, Box<dyn Biome + 'static>>, usize)>,
    Box<[CoordinateType; (CHUNK_DIMENSIONS * CHUNK_DIMENSIONS) as usize]>,
    BiomeIdList,
) {
    let mut biome_list = Box::new([0; CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE]);

    let mut biomes = vec![];

    let mut elevations = Box::new([0; CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE]);

    for i in 0..CHUNK_DIMENSIONS {
        for j in 0..CHUNK_DIMENSIONS {
            let seed_coords: BlockCoordinate = match up {
                BlockFace::Top => (i * scale + first_block_coord.x, s_dimensions, j * scale + first_block_coord.z),
                BlockFace::Bottom => (i * scale + first_block_coord.x, 0, j * scale + first_block_coord.z),
                BlockFace::Front => (i * scale + first_block_coord.x, j * scale + first_block_coord.y, s_dimensions),
                BlockFace::Back => (i * scale + first_block_coord.x, j * scale + first_block_coord.y, 0),
                BlockFace::Right => (s_dimensions, i * scale + first_block_coord.y, j * scale + first_block_coord.z),
                BlockFace::Left => (0, i * scale + first_block_coord.y, j * scale + first_block_coord.z),
            }
            .into();

            let (biome_params, top_height) = generate_height(
                biome_decider,
                structure_location,
                seed_coords,
                noise_generator,
                up,
                s_dimensions,
                sea_level,
            );

            elevations[flatten_2d(i as usize, j as usize, CHUNK_DIMENSIONS_USIZE)] = top_height;

            let idx = biosphere_biomes.ideal_biome_index_for(biome_params);

            biome_list[flatten_2d(i as usize, j as usize, CHUNK_DIMENSIONS as usize)] = idx as u8;

            if !biomes.iter().any(|(_, biome_idx)| idx == *biome_idx) {
                biomes.push((biosphere_biomes.biome_from_index(idx), idx));
            }
        }
    }

    (biomes, elevations, BiomeIdList::Face(biome_list))
}

const EDGE_SIZE: usize = CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * 2;

fn calculate_biomes_and_elevations_edge<'a, T: BiosphereMarkerComponent>(
    first_block_coord: BlockCoordinate,
    structure_location: &Location,
    noise_generator: &Noise,
    scale: CoordinateType,
    biome_decider: &BiomeDecider<T>,
    biosphere_biomes: &'a BiosphereBiomesRegistry<T>,
    sea_level: CoordinateType,
    j_up: BlockFace,
    k_up: BlockFace,
    s_dimensions: CoordinateType,
) -> (
    Vec<(RwLockReadGuard<'a, Box<dyn Biome + 'static>>, usize)>,
    Box<[CoordinateType; EDGE_SIZE]>,
    BiomeIdList,
) {
    let mut biome_list = Box::new([0; EDGE_SIZE]);

    let mut biomes = vec![];

    let mut elevations = Box::new([0; EDGE_SIZE]);

    for i in 0..CHUNK_DIMENSIONS {
        let i_scaled = i * scale;
        for j in 0..CHUNK_DIMENSIONS {
            let j_scaled = j as CoordinateType * scale;

            // Seed coordinates and j-direction noise functions.
            let (mut x, mut y, mut z) = (
                first_block_coord.x + i_scaled,
                first_block_coord.y + i_scaled,
                first_block_coord.z + i_scaled,
            );

            match j_up {
                BlockFace::Front => z = s_dimensions,
                BlockFace::Back => z = 0,
                BlockFace::Top => y = s_dimensions,
                BlockFace::Bottom => y = 0,
                BlockFace::Right => x = s_dimensions,
                BlockFace::Left => x = 0,
            };
            match k_up {
                BlockFace::Front | BlockFace::Back => z = first_block_coord.z + j_scaled,
                BlockFace::Top | BlockFace::Bottom => y = first_block_coord.y + j_scaled,
                BlockFace::Right | BlockFace::Left => x = first_block_coord.x + j_scaled,
            };

            let (biome_params, top_height) = generate_height(
                biome_decider,
                structure_location,
                BlockCoordinate::new(x, y, z),
                noise_generator,
                j_up,
                s_dimensions,
                sea_level,
            );

            let list_index = flatten(i as usize, j as usize, 0, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE);

            elevations[list_index] = top_height;

            let idx = biosphere_biomes.ideal_biome_index_for(biome_params);

            biome_list[list_index] = idx as u8;

            if !biomes.iter().any(|(_, biome_idx)| idx == *biome_idx) {
                biomes.push((biosphere_biomes.biome_from_index(idx), idx));
            }
        }

        for j in 0..CHUNK_DIMENSIONS {
            let j_scaled = j as CoordinateType * scale;

            // Seed coordinates and k-direction noise functions.
            let (mut x, mut y, mut z) = (
                first_block_coord.x + i_scaled,
                first_block_coord.y + i_scaled,
                first_block_coord.z + i_scaled,
            );
            match k_up {
                BlockFace::Front => z = s_dimensions,
                BlockFace::Back => z = 0,
                BlockFace::Top => y = s_dimensions,
                BlockFace::Bottom => y = 0,
                BlockFace::Right => x = s_dimensions,
                BlockFace::Left => x = 0,
            };
            match j_up {
                BlockFace::Front | BlockFace::Back => z = first_block_coord.z + j_scaled,
                BlockFace::Top | BlockFace::Bottom => y = first_block_coord.y + j_scaled,
                BlockFace::Right | BlockFace::Left => x = first_block_coord.x + j_scaled,
            };

            let (biome_params, top_height) = generate_height(
                biome_decider,
                structure_location,
                BlockCoordinate::new(x, y, z),
                noise_generator,
                k_up,
                s_dimensions,
                sea_level,
            );

            let list_index = flatten(i as usize, j as usize, 1, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE);

            elevations[list_index] = top_height;

            let idx = biosphere_biomes.ideal_biome_index_for(biome_params);

            biome_list[list_index] = idx as u8;

            if !biomes.iter().any(|(_, biome_idx)| idx == *biome_idx) {
                biomes.push((biosphere_biomes.biome_from_index(idx), idx));
            }
        }
    }

    (biomes, elevations, BiomeIdList::Edge(biome_list))
}

const CORNER_SIZE: usize = CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * 3;

// Might trim 45s, see generate_edge_chunk.
fn calculate_biomes_and_elevations_corner<'a, T: BiosphereMarkerComponent>(
    first_block_coord: BlockCoordinate,
    structure_location: &Location,
    noise_generator: &Noise,
    scale: CoordinateType,
    biome_decider: &BiomeDecider<T>,
    biosphere_biomes: &'a BiosphereBiomesRegistry<T>,
    sea_level: CoordinateType,
    x_up: BlockFace,
    y_up: BlockFace,
    z_up: BlockFace,
    s_dimensions: CoordinateType,
) -> (
    Vec<(RwLockReadGuard<'a, Box<dyn Biome + 'static>>, usize)>,
    Box<[CoordinateType; CORNER_SIZE]>,
    BiomeIdList,
) {
    let mut biome_list = Box::new([0; CORNER_SIZE]);

    let mut biomes = vec![];

    let mut elevations = Box::new([0; CORNER_SIZE]);

    // x top height cache.
    for y in 0..CHUNK_DIMENSIONS {
        let y_scaled = y * scale;
        for z in 0..CHUNK_DIMENSIONS {
            let z_scaled = z * scale;

            // Seed coordinates for the noise function.
            let seed_coords = match x_up {
                BlockFace::Right => (s_dimensions, first_block_coord.y + y_scaled, first_block_coord.z + z_scaled),
                _ => (0, first_block_coord.y + y_scaled, first_block_coord.z + z_scaled),
            }
            .into();

            // Unmodified top height.
            let (biome_params, top_height) = generate_height(
                biome_decider,
                structure_location,
                seed_coords,
                noise_generator,
                x_up,
                s_dimensions,
                sea_level,
            );

            let list_index = flatten(y as usize, z as usize, 0, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE);

            elevations[list_index] = top_height;

            let idx = biosphere_biomes.ideal_biome_index_for(biome_params);

            biome_list[list_index] = idx as u8;

            if !biomes.iter().any(|(_, biome_idx)| idx == *biome_idx) {
                biomes.push((biosphere_biomes.biome_from_index(idx), idx));
            }
        }
    }

    // y top height cache.
    for x in 0..CHUNK_DIMENSIONS {
        let x_scaled = x * scale;
        for z in 0..CHUNK_DIMENSIONS {
            let z_scaled = z * scale;

            // Seed coordinates for the noise function. Which loop variable goes to which xyz must agree everywhere.
            let seed_coords = match y_up {
                BlockFace::Top => (first_block_coord.x + x_scaled, s_dimensions, first_block_coord.z + z_scaled),
                _ => (first_block_coord.x + x_scaled, 0, first_block_coord.z + z_scaled),
            }
            .into();

            // Unmodified top height.
            let (biome_params, top_height) = generate_height(
                biome_decider,
                structure_location,
                seed_coords,
                noise_generator,
                y_up,
                s_dimensions,
                sea_level,
            );

            let list_index = flatten(x as usize, z as usize, 1, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE);

            elevations[list_index] = top_height;

            let idx = biosphere_biomes.ideal_biome_index_for(biome_params);

            biome_list[list_index] = idx as u8;

            if !biomes.iter().any(|(_, biome_idx)| idx == *biome_idx) {
                biomes.push((biosphere_biomes.biome_from_index(idx), idx));
            }
        }
    }

    for x in 0..CHUNK_DIMENSIONS {
        let x_scaled = x * scale;
        for y in 0..CHUNK_DIMENSIONS {
            let y_scaled = y * scale;

            // Seed coordinates for the noise function.
            let seed_coords = match z_up {
                BlockFace::Front => (first_block_coord.x + x_scaled, first_block_coord.y + y_scaled, s_dimensions),
                _ => (first_block_coord.x + x_scaled, first_block_coord.y + y_scaled, 0),
            }
            .into();

            // Unmodified top height.
            let (biome_params, top_height) = generate_height(
                biome_decider,
                structure_location,
                seed_coords,
                noise_generator,
                z_up,
                s_dimensions,
                sea_level,
            );

            let list_index = flatten(x as usize, y as usize, 2, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE);

            elevations[list_index] = top_height;

            let idx = biosphere_biomes.ideal_biome_index_for(biome_params);

            biome_list[list_index] = idx as u8;

            if !biomes.iter().any(|(_, biome_idx)| idx == *biome_idx) {
                biomes.push((biosphere_biomes.biome_from_index(idx), idx));
            }
        }
    }

    (biomes, elevations, BiomeIdList::Corner(biome_list))
}

fn generate<T: BiosphereMarkerComponent>(
    generating_lod: &mut GeneratingLod,
    structure_location: &Location,
    first_block_coord: BlockCoordinate,
    s_dimensions: CoordinateType,
    scale: CoordinateType,
    noise_generator: &Noise,
    biome_decider: &BiomeDecider<T>,
    biosphere_biomes: &BiosphereBiomesRegistry<T>,
    sea_level: Option<&BiosphereSeaLevel<T>>,
) {
    let mut lod_chunk = Box::new(LodChunk::new());
    let chunk_faces = Planet::chunk_planet_faces_with_scale(first_block_coord, s_dimensions, scale);

    let sea_level = sea_level.map(|x| ((x.level * s_dimensions as f32) as CoordinateType, x.block.as_ref()));
    let numeric_sea_level = sea_level.as_ref().map(|(level, _)| *level).unwrap_or(s_dimensions * 3 / 4);

    match chunk_faces {
        ChunkFaces::Face(up) => {
            let (biomes, elevations, biome_list) = calculate_biomes_and_elevations_face(
                first_block_coord,
                structure_location,
                noise_generator,
                scale,
                biome_decider,
                biosphere_biomes,
                numeric_sea_level,
                up,
                s_dimensions,
            );

            for (biome, biome_id) in biomes {
                let biome_id = biome_id as u8;

                biome.generate_face_chunk_lod(
                    biome.as_ref(),
                    first_block_coord,
                    s_dimensions,
                    &mut lod_chunk,
                    up,
                    scale,
                    &biome_list,
                    biome_id,
                    elevations.as_ref(),
                    sea_level,
                );
            }
        }
        ChunkFaces::Edge(j_up, k_up) => {
            let (biomes, elevations, biome_list) = calculate_biomes_and_elevations_edge(
                first_block_coord,
                structure_location,
                noise_generator,
                scale,
                biome_decider,
                biosphere_biomes,
                numeric_sea_level,
                j_up,
                k_up,
                s_dimensions,
            );

            for (biome, biome_id) in biomes {
                biome.generate_edge_chunk_lod(
                    biome.as_ref(),
                    first_block_coord,
                    s_dimensions,
                    &mut lod_chunk,
                    j_up,
                    k_up,
                    scale,
                    &biome_list,
                    biome_id as u8,
                    elevations.as_ref(),
                    sea_level,
                );
            }
        }
        ChunkFaces::Corner(x_up, y_up, z_up) => {
            let (biomes, elevations, biome_list) = calculate_biomes_and_elevations_corner(
                first_block_coord,
                structure_location,
                noise_generator,
                scale,
                biome_decider,
                biosphere_biomes,
                numeric_sea_level,
                x_up,
                y_up,
                z_up,
                s_dimensions,
            );

            for (biome, biome_id) in biomes {
                biome.generate_corner_chunk_lod(
                    biome.as_ref(),
                    first_block_coord,
                    s_dimensions,
                    &mut lod_chunk,
                    x_up,
                    y_up,
                    z_up,
                    scale,
                    &biome_list,
                    biome_id as u8,
                    elevations.as_ref(),
                    sea_level,
                );
            }
        }
    }

    *generating_lod = GeneratingLod::DoneGenerating(lod_chunk);
}

fn recurse<T: BiosphereMarkerComponent>(
    generating_lod: &mut GeneratingLod,
    structure_location: &Location,
    first_block_coord: BlockCoordinate,
    s_dimensions: CoordinateType,
    scale: CoordinateType,
    noise_generator: &Noise,
    biome_decider: &BiomeDecider<T>,
    biosphere_biomes: &BiosphereBiomesRegistry<T>,
    sea_level: Option<&BiosphereSeaLevel<T>>,
) {
    match generating_lod {
        GeneratingLod::NeedsGenerated => {
            *generating_lod = GeneratingLod::BeingGenerated;
            generate::<T>(
                generating_lod,
                structure_location,
                first_block_coord,
                s_dimensions,
                scale,
                noise_generator,
                biome_decider,
                biosphere_biomes,
                sea_level,
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
                recurse::<T>(
                    child,
                    structure_location,
                    BlockCoordinate::new(bx, by, bz) + first_block_coord,
                    s_dimensions,
                    s2,
                    noise_generator,
                    biome_decider,
                    biosphere_biomes,
                    sea_level,
                );
            });
        }
        _ => {}
    }
}

pub(crate) fn begin_generating_lods<T: BiosphereMarkerComponent>(
    query: Query<(Entity, &LodNeedsGeneratedForPlayer), With<T>>,
    is_biosphere: Query<(&Structure, &Location), With<T>>,
    noise_generator: Res<ReadOnlyNoise>,
    mut currently_generating: ResMut<GeneratingLods<T>>,
    mut commands: Commands,
    biosphere_biomes: Res<BiosphereBiomesRegistry<T>>,
    biome_decider: Res<BiomeDecider<T>>,
    sea_level: Option<Res<BiosphereSeaLevel<T>>>,
) {
    let sea_level = sea_level.map(|x| x.clone());

    for (entity, generating_lod) in query.iter() {
        commands.entity(entity).despawn_recursive();

        let Ok((structure, location)) = is_biosphere.get(generating_lod.structure_entity) else {
            return;
        };

        let (player_entity, structure_entity) = (generating_lod.player_entity, generating_lod.structure_entity);

        let task_pool = AsyncComputeTaskPool::get();

        let dimensions = structure.block_dimensions().x;

        let mut generating_lod = generating_lod.clone();
        let noise_generator = noise_generator.clone();

        let biome_decider = *biome_decider;
        let biosphere_biomes = biosphere_biomes.clone();
        let location = *location;
        let sea_level = sea_level.clone();

        let task = task_pool.spawn(async move {
            let noise = noise_generator.inner();

            let first_block_coord = BlockCoordinate::new(0, 0, 0);

            recurse::<T>(
                &mut generating_lod.generating_lod,
                &location,
                first_block_coord,
                dimensions,
                dimensions / CHUNK_DIMENSIONS,
                &noise,
                &biome_decider,
                &biosphere_biomes,
                sea_level.as_ref(),
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
pub fn generate_planet<T: BiosphereMarkerComponent, E: TGenerateChunkEvent>(
    mut query: Query<(&mut Structure, &Location)>,
    mut generating: ResMut<GeneratingChunks<T>>,
    mut events: EventReader<E>,
    noise_generator: Res<ReadOnlyNoise>,
    biosphere_biomes: Res<BiosphereBiomesRegistry<T>>,
    biome_decider: Res<BiomeDecider<T>>,
    sea_level: Option<Res<BiosphereSeaLevel<T>>>,
) {
    let chunks = events
        .read()
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
        for (mut chunk, s_dimensions, structure_location, structure_entity) in chunks {
            let sea_level = sea_level
                .as_ref()
                .map(|x| ((x.level * s_dimensions as f32) as CoordinateType, x.block.clone()));
            let numeric_sea_level = sea_level.as_ref().map(|(level, _)| *level).unwrap_or(s_dimensions * 3 / 4);

            let noise = noise_generator.clone();

            let biome_decider = *biome_decider;
            let biosphere_biomes = biosphere_biomes.clone();

            let task = thread_pool.spawn(async move {
                let noise_generator = noise.inner();
                // let timer = UtilsTimer::start();

                // To save multiplication operations later.
                let first_block_coord = chunk.chunk_coordinates().first_structure_block();

                // Get all possible planet faces from the chunk corners.
                let chunk_faces = Planet::chunk_planet_faces(first_block_coord, s_dimensions);

                let sea_level_ref = sea_level.as_ref().map(|x| (x.0, x.1.as_ref()));
                match chunk_faces {
                    ChunkFaces::Face(up) => {
                        let (biomes, elevations, biome_list) = calculate_biomes_and_elevations_face::<T>(
                            first_block_coord,
                            &structure_location,
                            &noise_generator,
                            1,
                            &biome_decider,
                            &biosphere_biomes,
                            numeric_sea_level,
                            up,
                            s_dimensions,
                        );

                        for (biome, biome_id) in biomes {
                            let biome_id = biome_id as u8;

                            biome.generate_face_chunk(
                                biome.as_ref(),
                                first_block_coord,
                                s_dimensions,
                                &mut chunk,
                                up,
                                &biome_list,
                                biome_id,
                                elevations.as_ref(),
                                sea_level_ref,
                            );
                        }
                    }
                    ChunkFaces::Edge(j_up, k_up) => {
                        let (biomes, elevations, biome_id_list) = calculate_biomes_and_elevations_edge::<T>(
                            first_block_coord,
                            &structure_location,
                            &noise_generator,
                            1,
                            &biome_decider,
                            &biosphere_biomes,
                            numeric_sea_level,
                            j_up,
                            k_up,
                            s_dimensions,
                        );

                        for (biome, biome_id) in biomes {
                            biome.generate_edge_chunk(
                                biome.as_ref(),
                                first_block_coord,
                                s_dimensions,
                                &mut chunk,
                                j_up,
                                k_up,
                                &biome_id_list,
                                biome_id as u8,
                                elevations.as_ref(),
                                sea_level_ref,
                            );
                        }
                    }
                    ChunkFaces::Corner(x_up, y_up, z_up) => {
                        let (biomes, elevations, biome_list) = calculate_biomes_and_elevations_corner::<T>(
                            first_block_coord,
                            &structure_location,
                            &noise_generator,
                            1,
                            &biome_decider,
                            &biosphere_biomes,
                            numeric_sea_level,
                            x_up,
                            y_up,
                            z_up,
                            s_dimensions,
                        );

                        for (biome, biome_id) in biomes {
                            biome.generate_corner_chunk(
                                biome.as_ref(),
                                first_block_coord,
                                s_dimensions,
                                &mut chunk,
                                x_up,
                                y_up,
                                z_up,
                                &biome_list,
                                biome_id as u8,
                                elevations.as_ref(),
                                sea_level_ref,
                            );
                        }
                    }
                }
                // timer.log_duration("Chunk:");
                (chunk, structure_entity)
            });

            generating.generating.push(GeneratingChunk::new(task));
        }
    }
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
    noise_generator: &Noise,
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

const GUIDE_MIN: CoordinateType = 100;

/// Generates the "old" height, the one that's used if you're in the middle of a face.
/// Also generates the height at any edge within GUIDE_MIN distance.
/// Averages the "old" height with the edge heights with coefficients based on how close you are to the edge intersection.
fn guide(
    noise_generator: &Noise,
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
        x_height = get_block_height(
            noise_generator,
            x_seed,
            structure_coords,
            middle_air_start,
            amplitude,
            delta,
            iterations,
        );
        x_coefficient = get_mirror_coefficient(block_coords.x, x_height as CoordinateType, s_dimensions);
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
        y_height = get_block_height(
            noise_generator,
            y_seed,
            structure_coords,
            middle_air_start,
            amplitude,
            delta,
            iterations,
        );
        y_coefficient = get_mirror_coefficient(block_coords.y, y_height as CoordinateType, s_dimensions);
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
        z_height = get_block_height(
            noise_generator,
            z_seed,
            structure_coords,
            middle_air_start,
            amplitude,
            delta,
            iterations,
        );
        z_coefficient = get_mirror_coefficient(block_coords.z, z_height as CoordinateType, s_dimensions);
    }

    match block_up {
        BlockFace::Front | BlockFace::Back => merge(z_height, x_coefficient, x_height, y_coefficient, y_height),
        BlockFace::Top | BlockFace::Bottom => merge(y_height, x_coefficient, x_height, z_coefficient, z_height),
        BlockFace::Right | BlockFace::Left => merge(x_height, y_coefficient, y_height, z_coefficient, z_height),
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
    noise_generator: &Noise,
    middle_air_start: CoordinateType,
    amplitude: f64,
    delta: f64,
    iterations: usize,
) -> CoordinateType {
    guide(
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
