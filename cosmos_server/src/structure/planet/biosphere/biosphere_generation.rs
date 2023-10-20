//! Responsible for the default generation of biospheres.

use std::{marker::PhantomData, mem::swap};

use bevy::{
    prelude::{warn, Commands, DespawnRecursiveExt, Entity, Event, EventReader, EventWriter, Query, Res, ResMut, Resource, With},
    tasks::AsyncComputeTaskPool,
};
use cosmos_core::{
    block::Block,
    netty::cosmos_encoder,
    physics::location::Location,
    registry::Registry,
    structure::{
        chunk::{Chunk, CHUNK_DIMENSIONS},
        coordinates::{BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate, CoordinateType},
        lod::{LodDelta, LodNetworkMessage, SetLodMessage},
        lod_chunk::LodChunk,
        planet::{ChunkFaces, Planet},
        Structure,
    },
};
use futures_lite::future;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::{
    init::init_world::{Noise, ReadOnlyNoise},
    structure::planet::lods::generate_lods::{
        AsyncGeneratingLod, DoneGeneratingLod, GeneratingLod, GeneratingLods, LodNeedsGeneratedForPlayer,
    },
};

use super::{
    biome::{Biome, BiosphereBiomesRegistry},
    BiomeDecider, BiosphereMarkerComponent, GeneratingChunk, GeneratingChunks, TGenerateChunkEvent,
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
    sea_block: Option<(CoordinateType, Block)>,
}

impl BlockLayers {
    pub fn ranges(&self) -> std::slice::Iter<(cosmos_core::block::Block, BlockLayer)> {
        self.ranges.iter()
    }

    pub fn sea_level(&self) -> Option<&(CoordinateType, Block)> {
        self.sea_block.as_ref()
    }
}

/// Stores the blocks and all the noise information for creating the top of their layer.
/// For example, the "stone" BlockLevel has the noise paramters that create the boundry between dirt and stone.
#[derive(Clone, Debug)]
pub struct BlockLayer {
    pub middle_depth: CoordinateType,
    pub delta: f64,
    pub amplitude: f64,
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

    /// Sets the sea level and the block that goes along with it
    pub fn with_sea_level_block(
        mut self,
        block_id: &str,
        block_registry: &Registry<Block>,
        sea_level: CoordinateType,
    ) -> Result<Self, BlockRangeError> {
        let Some(block) = block_registry.from_id(block_id).cloned() else {
            return Err(BlockRangeError::MissingBlock(self));
        };
        self.sea_block = Some((sea_level, block));
        Ok(self)
    }

    pub fn face_block<'a>(
        &self,
        height: CoordinateType,
        block_layers: &[(&'a Block, CoordinateType)],
        sea_level: Option<&'a (CoordinateType, Block)>,
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
        if let Some((sea_level, sea_block)) = sea_level {
            if height <= *sea_level {
                Some(sea_block)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn edge_block<'a>(
        &self,
        j_height: CoordinateType,
        k_height: CoordinateType,
        j_layers: &[(&'a Block, CoordinateType)],
        k_layers: &[(&'a Block, CoordinateType)],
        sea_level: Option<&'a (CoordinateType, Block)>,
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
        if let Some((sea_level, sea_block)) = sea_level {
            if j_height.max(k_height) <= *sea_level {
                Some(sea_block)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn corner_block<'a>(
        &self,
        x_height: CoordinateType,
        y_height: CoordinateType,
        z_height: CoordinateType,
        x_layers: &[(&'a Block, CoordinateType)],
        y_layers: &[(&'a Block, CoordinateType)],
        z_layers: &[(&'a Block, CoordinateType)],
        sea_level: Option<&'a (CoordinateType, Block)>,
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

        if let Some((sea_level, sea_block)) = sea_level {
            if x_height.max(y_height).max(z_height) <= *sea_level {
                Some(sea_block)
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn generate(
    generating_lod: &mut GeneratingLod,
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    first_block_coord: BlockCoordinate,
    s_dimensions: CoordinateType,
    scale: CoordinateType,
    noise_generator: &noise::OpenSimplex,
    biome: &dyn Biome,
) {
    let mut lod_chunk = Box::new(LodChunk::new());

    let chunk_faces = Planet::chunk_planet_faces_with_scale(first_block_coord, s_dimensions, scale);
    match chunk_faces {
        ChunkFaces::Face(up) => {
            biome.generate_face_chunk_lod(
                biome,
                first_block_coord,
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                &mut lod_chunk,
                up,
                scale,
                ChunkBlockCoordinate::min(),
                ChunkBlockCoordinate::max(),
            );
        }
        ChunkFaces::Edge(j_up, k_up) => {
            biome.generate_edge_chunk_lod(
                biome,
                first_block_coord,
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                &mut lod_chunk,
                j_up,
                k_up,
                scale,
                ChunkBlockCoordinate::min(),
                ChunkBlockCoordinate::max(),
            );
        }
        ChunkFaces::Corner(x_up, y_up, z_up) => {
            biome.generate_corner_chunk_lod(
                biome,
                first_block_coord,
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                &mut lod_chunk,
                x_up,
                y_up,
                z_up,
                scale,
                ChunkBlockCoordinate::min(),
                ChunkBlockCoordinate::max(),
            );
        }
    }

    // lod_chunk.fill(blocks.from_id("cosmos:grass").expect("Missing grass!"), BlockFace::Top);
    *generating_lod = GeneratingLod::DoneGenerating(lod_chunk);
}

fn recurse<T: BiosphereMarkerComponent>(
    generating_lod: &mut GeneratingLod,
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    first_block_coord: BlockCoordinate,
    s_dimensions: CoordinateType,
    scale: CoordinateType,
    noise_generator: &Noise,
    biome: &dyn Biome,
) {
    match generating_lod {
        GeneratingLod::NeedsGenerated => {
            *generating_lod = GeneratingLod::BeingGenerated;
            generate(
                generating_lod,
                (structure_x, structure_y, structure_z),
                first_block_coord,
                s_dimensions,
                scale,
                noise_generator,
                biome,
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
                    (structure_x, structure_y, structure_z),
                    BlockCoordinate::new(bx, by, bz) + first_block_coord,
                    s_dimensions,
                    s2,
                    noise_generator,
                    biome,
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

        let biome_decider = *biome_decider;
        let biosphere_biomes = biosphere_biomes.clone();
        let location = *location;

        let task = task_pool.spawn(async move {
            let noise = noise_generator.inner();

            let first_block_coord = BlockCoordinate::new(0, 0, 0);

            let biome_params = biome_decider.biome_parameters_at(&location, first_block_coord, &noise);

            let biome = biosphere_biomes.ideal_biome_for(biome_params);

            recurse::<T>(
                &mut generating_lod.generating_lod,
                (structure_coords.x, structure_coords.y, structure_coords.z),
                first_block_coord,
                dimensions,
                dimensions / CHUNK_DIMENSIONS,
                &noise,
                biome.as_ref(),
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
            let noise = noise_generator.clone();

            let biome_decider = *biome_decider;
            let biosphere_biomes = biosphere_biomes.clone();

            let task = thread_pool.spawn(async move {
                let noise_generator = noise.inner();
                // let timer = UtilsTimer::start();

                let actual_pos = location.absolute_coords_f64();

                let structure_z = actual_pos.z;
                let structure_y = actual_pos.y;
                let structure_x = actual_pos.x;

                // To save multiplication operations later.
                let first_block_coord = chunk.chunk_coordinates().first_structure_block();

                let biome_params = biome_decider.biome_parameters_at(&location, first_block_coord, &noise_generator);

                let biome = biosphere_biomes.ideal_biome_for(biome_params);

                // Get all possible planet faces from the chunk corners.
                let chunk_faces = Planet::chunk_planet_faces(first_block_coord, s_dimensions);
                match chunk_faces {
                    ChunkFaces::Face(up) => {
                        biome.generate_face_chunk(
                            biome.as_ref(),
                            first_block_coord,
                            (structure_x, structure_y, structure_z),
                            s_dimensions,
                            &noise_generator,
                            &mut chunk,
                            up,
                            1,
                            ChunkBlockCoordinate::min(),
                            ChunkBlockCoordinate::max(),
                        );
                    }
                    ChunkFaces::Edge(j_up, k_up) => {
                        biome.generate_edge_chunk(
                            biome.as_ref(),
                            first_block_coord,
                            (structure_x, structure_y, structure_z),
                            s_dimensions,
                            &noise_generator,
                            &mut chunk,
                            j_up,
                            k_up,
                            1,
                            ChunkBlockCoordinate::min(),
                            ChunkBlockCoordinate::max(),
                        );
                    }
                    ChunkFaces::Corner(x_up, y_up, z_up) => {
                        biome.generate_corner_chunk(
                            biome.as_ref(),
                            first_block_coord,
                            (structure_x, structure_y, structure_z),
                            s_dimensions,
                            &noise_generator,
                            &mut chunk,
                            x_up,
                            y_up,
                            z_up,
                            1,
                            ChunkBlockCoordinate::min(),
                            ChunkBlockCoordinate::max(),
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
