//! Contains logic related to the localized formation of terrain

use std::{
    hash::Hash,
    marker::PhantomData,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use bevy::prelude::{info, App, EventWriter, OnExit, ResMut, Resource, Vec3};
use cosmos_core::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::Registry,
    structure::{
        block_storage::BlockStorer,
        chunk::{Chunk, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS_USIZE},
        coordinates::{BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate, CoordinateType},
        lod_chunk::LodChunk,
        planet::Planet,
        Structure,
    },
    utils::array_utils::{flatten, flatten_2d},
};
use noise::NoiseFn;

use crate::{
    init::init_world::{Noise, ServerSeed},
    state::GameState,
};

use super::{biosphere_generation::BlockLayers, BiosphereMarkerComponent};

pub mod biome_registry;
pub mod desert;
pub mod ocean;
pub mod plains;

#[inline]
fn generate_face_chunk<C: BlockStorer>(
    biome: &dyn Biome,
    block_coords: BlockCoordinate,
    structure_coords: (f64, f64, f64),
    s_dimensions: CoordinateType,
    noise_generator: &Noise,
    chunk: &mut C,
    up: BlockFace,
    scale: CoordinateType,
    biome_id_list: &BiomeIdList,
    self_biome_id: u8,
    elevation: &[CoordinateType; (CHUNK_DIMENSIONS * CHUNK_DIMENSIONS) as usize],
    sea_level: Option<&(CoordinateType, Block)>,
) {
    let BiomeIdList::Face(biome_id_list) = biome_id_list else {
        panic!("Invalid biome id list type passed!");
    };

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

            let elevation = elevation[flatten_2d(i as usize, j as usize, CHUNK_DIMENSIONS as usize)];

            let mut depth_increase = 0;

            let concrete_ranges = block_layers
                .ranges()
                .map(|(block, level)| {
                    let layer_height = elevation - level.middle_depth - depth_increase;

                    // for _ in 0..level.iterations {
                    //     layer_height += (level.amplitude
                    //         * noise_generator
                    //             .get([
                    //                 (structure_coords.0 + seed_coords.x as f64) * level.delta,
                    //                 (structure_coords.1 + seed_coords.y as f64) * level.delta,
                    //                 (structure_coords.2 + seed_coords.z as f64) * level.delta,
                    //             ])
                    //             .round()) as CoordinateType;
                    // }

                    depth_increase += level.middle_depth;

                    (block, layer_height)
                })
                .collect::<Vec<(&Block, CoordinateType)>>();

            for chunk_height in 0..CHUNK_DIMENSIONS {
                let coords: ChunkBlockCoordinate = match up {
                    BlockFace::Front | BlockFace::Back => (i, j, chunk_height),
                    BlockFace::Top | BlockFace::Bottom => (i, chunk_height, j),
                    BlockFace::Right | BlockFace::Left => (chunk_height, i, j),
                }
                .into();

                if biome_id_list[flatten_2d(i as usize, j as usize, CHUNK_DIMENSIONS_USIZE)] != self_biome_id {
                    continue;
                }

                let height = match up {
                    BlockFace::Front => sz + chunk_height * scale,
                    BlockFace::Back => s_dimensions - (sz + chunk_height * scale),
                    BlockFace::Top => sy + chunk_height * scale,
                    BlockFace::Bottom => s_dimensions - (sy + chunk_height * scale),
                    BlockFace::Right => sx + chunk_height * scale,
                    BlockFace::Left => s_dimensions - (sx + chunk_height * scale),
                };

                // if scale == 1 && height >= 1530 && height <= 1560 {
                //     println!("HEIGHT: {height}");
                //     println!("RANGES: {concrete_ranges:?}");
                //     println!("Elevatinon: {elevation}");
                // }

                let block = block_layers.face_block(height, &concrete_ranges, sea_level, scale);
                if let Some(block) = block {
                    chunk.set_block_at(coords, block, up);
                }
            }
        }
    }
}

fn generate_edge_chunk<C: BlockStorer>(
    biome: &dyn Biome,
    block_coords: BlockCoordinate,
    structure_coords: (f64, f64, f64),
    s_dimensions: CoordinateType,
    noise_generator: &Noise,
    chunk: &mut C,
    j_up: BlockFace,
    k_up: BlockFace,
    scale: CoordinateType,
    biome_id_list: &BiomeIdList,
    self_biome_id: u8,
    elevation: &[CoordinateType; CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * 2],
    sea_level: Option<&(CoordinateType, Block)>,
) {
    let BiomeIdList::Edge(biome_id_list) = biome_id_list else {
        panic!("Invalid biome id list type passed!");
    };

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

            let elevation = elevation[flatten(i as usize, j as usize, 0, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE)];

            let mut depth_increase = 0;
            for (block, level) in block_layers.ranges() {
                let layer_height = elevation - level.middle_depth - depth_increase;

                // for _ in 0..level.iterations {
                //     layer_height += (level.amplitude
                //         * noise_generator
                //             .get([
                //                 (structure_coords.0 + seed_coords.x as f64) * level.delta,
                //                 (structure_coords.1 + seed_coords.y as f64) * level.delta,
                //                 (structure_coords.2 + seed_coords.z as f64) * level.delta,
                //             ])
                //             .round()) as CoordinateType;
                // }

                depth_increase += level.middle_depth;

                j_layers.push((block, layer_height));
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
            // for (block, layer) in block_layers.ranges() {
            //     let layer_top = biome.get_top_height(
            //         k_up,
            //         BlockCoordinate::new(x, y, z),
            //         structure_coords,
            //         s_dimensions,
            //         noise_generator,
            //         height - layer.middle_depth,
            //         layer.amplitude,
            //         layer.delta,
            //         layer.iterations,
            //     );
            //     k_layers.push((block, layer_top));
            //     height = layer_top;
            // }

            let elevation = elevation[flatten(i as usize, j as usize, 1, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE)];

            let mut depth_increase = 0;
            for (block, level) in block_layers.ranges() {
                let layer_height = elevation - level.middle_depth - depth_increase;

                // for _ in 0..level.iterations {
                //     layer_height += (level.amplitude
                //         * noise_generator
                //             .get([
                //                 (structure_coords.0 + seed_coords.x as f64) * level.delta,
                //                 (structure_coords.1 + seed_coords.y as f64) * level.delta,
                //                 (structure_coords.2 + seed_coords.z as f64) * level.delta,
                //             ])
                //             .round()) as CoordinateType;
                // }

                depth_increase += level.middle_depth;

                k_layers.push((block, layer_height));
            }

            if j_layers_cache[j as usize][0].1 == j_height && k_layers[0].1 == j_height && first_both_45 == s_dimensions {
                first_both_45 = j_height;
            }

            for (k, j_layers) in j_layers_cache.iter().enumerate() {
                let mut chunk_block_coords = ChunkBlockCoordinate::new(i, i, i);

                let block_up = Planet::get_planet_face_without_structure(
                    BlockCoordinate::new(
                        block_coords.x + chunk_block_coords.x * scale,
                        block_coords.y + chunk_block_coords.y * scale,
                        block_coords.z + chunk_block_coords.z * scale,
                    ),
                    s_dimensions,
                );

                if biome_id_list[flatten(
                    i as usize,
                    j as usize,
                    if block_up == j_up { 1 } else { 0 },
                    CHUNK_DIMENSIONS_USIZE,
                    CHUNK_DIMENSIONS_USIZE,
                )] != self_biome_id
                {
                    continue;
                }

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
    noise_generator: &Noise,
    chunk: &mut C,
    x_up: BlockFace,
    y_up: BlockFace,
    z_up: BlockFace,
    scale: CoordinateType,
    biome_id_list: &BiomeIdList,
    self_biome_id: u8,
) {
    return;
    // let block_layers = biome.block_layers();

    // // x top height cache.
    // let mut x_layers: Vec<Vec<(&Block, CoordinateType)>> = vec![vec![]; CHUNK_DIMENSIONS as usize * CHUNK_DIMENSIONS as usize];
    // for j in 0..CHUNK_DIMENSIONS {
    //     let j_scaled = j * scale;
    //     for k in 0..CHUNK_DIMENSIONS {
    //         let k_scaled = k * scale;

    //         let index = flatten_2d(j as usize, k as usize, CHUNK_DIMENSIONS as usize);

    //         // Seed coordinates for the noise function.
    //         let seed_coords = match x_up {
    //             BlockFace::Right => (s_dimensions, block_coords.y + j_scaled, block_coords.z + k_scaled),
    //             _ => (0, block_coords.y + j_scaled, block_coords.z + k_scaled),
    //         }
    //         .into();

    //         // Unmodified top height.
    //         let mut height = s_dimensions;
    //         for (block, level) in block_layers.ranges() {
    //             let level_top = biome.get_top_height(
    //                 x_up,
    //                 seed_coords,
    //                 structure_coords,
    //                 s_dimensions,
    //                 noise_generator,
    //                 height - level.middle_depth,
    //                 level.amplitude,
    //                 level.delta,
    //                 level.iterations,
    //             );
    //             x_layers[index].push((block, level_top));
    //             height = level_top;
    //         }
    //     }
    // }

    // // y top height cache.
    // let mut y_layers: Vec<Vec<(&Block, CoordinateType)>> = vec![vec![]; CHUNK_DIMENSIONS as usize * CHUNK_DIMENSIONS as usize];
    // for i in 0..CHUNK_DIMENSIONS {
    //     let i_scaled = i * scale;
    //     for k in 0..CHUNK_DIMENSIONS {
    //         let k_scaled = k * scale;

    //         let index = flatten_2d(i as usize, k as usize, CHUNK_DIMENSIONS as usize);

    //         // Seed coordinates for the noise function. Which loop variable goes to which xyz must agree everywhere.
    //         let seed_coords = match y_up {
    //             BlockFace::Top => (block_coords.x + i_scaled, s_dimensions, block_coords.z + k_scaled),
    //             _ => (block_coords.x + i_scaled, 0, block_coords.z + k_scaled),
    //         }
    //         .into();

    //         // Unmodified top height.
    //         let mut height = s_dimensions;
    //         for (block, level) in block_layers.ranges() {
    //             let level_top = biome.get_top_height(
    //                 y_up,
    //                 seed_coords,
    //                 structure_coords,
    //                 s_dimensions,
    //                 noise_generator,
    //                 height - level.middle_depth,
    //                 level.amplitude,
    //                 level.delta,
    //                 level.iterations,
    //             );
    //             y_layers[index].push((block, level_top));
    //             height = level_top;
    //         }
    //     }
    // }

    // for i in 0..CHUNK_DIMENSIONS {
    //     let i_scaled = i * scale;
    //     for j in 0..CHUNK_DIMENSIONS {
    //         let j_scaled = j * scale;

    //         // Seed coordinates for the noise function.
    //         let seed_coords = match z_up {
    //             BlockFace::Front => (block_coords.x + i_scaled, block_coords.y + j_scaled, s_dimensions),
    //             _ => (block_coords.x + i_scaled, block_coords.y + j_scaled, 0),
    //         }
    //         .into();

    //         // Unmodified top height.
    //         let mut height = s_dimensions;
    //         let mut z_layers = vec![];
    //         for (block, level) in block_layers.ranges() {
    //             let level_top = biome.get_top_height(
    //                 z_up,
    //                 seed_coords,
    //                 structure_coords,
    //                 s_dimensions,
    //                 noise_generator,
    //                 height - level.middle_depth,
    //                 level.amplitude,
    //                 level.delta,
    //                 level.iterations,
    //             );
    //             z_layers.push((block, level_top));
    //             height = level_top;
    //         }

    //         for k in 0..CHUNK_DIMENSIONS {
    //             let coords = ChunkBlockCoordinate::new(i, j, k);

    //             if biome_id_list.biome_id(coords) != self_biome_id {
    //                 continue;
    //             }

    //             let k_scaled = k * scale;

    //             let z_height = match z_up {
    //                 BlockFace::Front => block_coords.z + k_scaled,
    //                 _ => s_dimensions - (block_coords.z + k_scaled),
    //             };
    //             let y_height = match y_up {
    //                 BlockFace::Top => block_coords.y + j_scaled,
    //                 _ => s_dimensions - (block_coords.y + j_scaled),
    //             };
    //             let x_height = match x_up {
    //                 BlockFace::Right => block_coords.x + i_scaled,
    //                 _ => s_dimensions - (block_coords.x + i_scaled),
    //             };

    //             let block_up = Planet::get_planet_face_without_structure(
    //                 BlockCoordinate::new(block_coords.x + i_scaled, block_coords.y + j_scaled, block_coords.z + k_scaled),
    //                 s_dimensions,
    //             );
    //             let block = block_layers.corner_block(
    //                 x_height,
    //                 y_height,
    //                 z_height,
    //                 &x_layers[flatten_2d(j as usize, k as usize, CHUNK_DIMENSIONS as usize)],
    //                 &y_layers[flatten_2d(i as usize, k as usize, CHUNK_DIMENSIONS as usize)],
    //                 &z_layers,
    //                 block_layers.sea_level(),
    //                 scale,
    //             );
    //             if let Some(block) = block {
    //                 chunk.set_block_at(coords, block, block_up);
    //             }
    //         }
    //     }
    // }
}

/// This is used when generating chunks for both LODs and normally.
///
/// This maps every block in a chunk to its biome, based on the biome's "id". The id is just its index
/// in the [`BiosphereBiomesRegistry<T>`] where `T` is the biosphere.
///
/// This is mostly used to keep performance to a maximum.
pub enum BiomeIdList {
    Face(Box<[u8; (CHUNK_DIMENSIONS * CHUNK_DIMENSIONS) as usize]>),
    Edge(Box<[u8; (CHUNK_DIMENSIONS * CHUNK_DIMENSIONS * 2) as usize]>),
    Corner(Box<[u8; (CHUNK_DIMENSIONS * CHUNK_DIMENSIONS * 3) as usize]>),
}

/// A biome is a structure that dictates how terrain will be generated.
///
/// Biomes can be linked to biospheres, which will then call their methods to generate their terrain.
///
/// Biomes don't do anything, until registered in the [`BiosphereBiomesRegistry<T>`] where `T` is the biosphere they belong to.
///
/// Most methods in here don't need to be modified, and will work for most biome implementations.
/// The main ones to mess with are:
/// `id, unlocailized_name, set_numeric_id, block_layers`.
pub trait Biome: Send + Sync + 'static {
    /// Same as [`Identifiable::id`]
    fn id(&self) -> u16;
    /// Same as [`Identifiable::unlocalized_name`]
    fn unlocalized_name(&self) -> &str;
    /// Same as [`Identifiable::set_numeric_id`]
    fn set_numeric_id(&mut self, id: u16);

    /// Gets the block layers that this biome uses. Note that this is only used internally. If you don't need them, feel free to return an empty BlockLayers.
    fn block_layers(&self) -> &BlockLayers;

    /// Generates an lod chunk that is completely on one side of the planet
    /// - `self_as_dyn` Self as a dyn Biome pointer
    /// - `block_coords` The bottom-left-back-most coordinates to start the generation
    /// - `structure_coords` Just used to seed the noise function
    /// - `chunk` The chunk to fill in
    /// - `up` The up direction of this face
    /// - `scale` The scale of this LOD
    /// - `biome_id_list` A list of biomes each block in the lod chunk is
    /// - `self_biome_id` This biome's id for this specific biosphere. Used to check against itself in the `biome_id_list`
    fn generate_face_chunk_lod(
        &self,
        self_as_dyn: &dyn Biome,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &Noise,
        chunk: &mut LodChunk,
        up: BlockFace,
        scale: CoordinateType,
        biome_id_list: &BiomeIdList,
        self_biome_id: u8,
        elevation: &[CoordinateType; (CHUNK_DIMENSIONS * CHUNK_DIMENSIONS) as usize],
        sea_level: Option<&(CoordinateType, Block)>,
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
            biome_id_list,
            self_biome_id,
            elevation,
            sea_level,
        );
    }

    /// Generates an lod chunk that is on an edge of the planet
    /// - `self_as_dyn` Self as a dyn Biome pointer
    /// - `block_coords` The bottom-left-back-most coordinates to start the generation
    /// - `structure_coords` Just used to seed the noise function
    /// - `chunk` The chunk to fill in
    /// - `j_up` The up direction of one of the faces
    /// - `k_up` The up direction of the other of the faces
    /// - `scale` The scale of this LOD
    /// - `biome_id_list` A list of biomes each block in the lod chunk is
    /// - `self_biome_id` This biome's id for this specific biosphere. Used to check against itself in the `biome_id_list`
    fn generate_edge_chunk_lod(
        &self,
        self_as_dyn: &dyn Biome,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &Noise,
        chunk: &mut LodChunk,
        j_up: BlockFace,
        k_up: BlockFace,
        scale: CoordinateType,
        biome_id_list: &BiomeIdList,
        self_biome_id: u8,
        elevation: &[CoordinateType; CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * 2],
        sea_level: Option<&(CoordinateType, Block)>,
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
            biome_id_list,
            self_biome_id,
            elevation,
            sea_level,
        );
    }

    /// Generates an lod chunk that is on a corner of the planet
    /// - `self_as_dyn` Self as a dyn Biome pointer
    /// - `block_coords` The bottom-left-back-most coordinates to start the generation
    /// - `structure_coords` Just used to seed the noise function
    /// - `chunk` The chunk to fill in
    /// - `x_up` The up direction of the x face
    /// - `y_up` The up direction of the y face
    /// - `z_up` The up direction of the z face
    /// - `scale` The scale of this LOD
    /// - `biome_id_list` A list of biomes each block in the lod chunk is
    /// - `self_biome_id` This biome's id for this specific biosphere. Used to check against itself in the `biome_id_list`
    fn generate_corner_chunk_lod(
        &self,
        self_as_dyn: &dyn Biome,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &Noise,
        chunk: &mut LodChunk,
        x_up: BlockFace,
        y_up: BlockFace,
        z_up: BlockFace,
        scale: CoordinateType,
        biome_id_list: &BiomeIdList,
        self_biome_id: u8,
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
            biome_id_list,
            self_biome_id,
        );
    }

    /// Generates a chunk that is completely on one side of the planet
    /// - `self_as_dyn` Self as a dyn Biome pointer
    /// - `block_coords` The bottom-left-back-most coordinates to start the generation
    /// - `structure_coords` Just used to seed the noise function
    /// - `chunk` The chunk to fill in
    /// - `up` The up direction of this face
    /// - `biome_id_list` A list of biomes each block in the chunk is
    /// - `self_biome_id` This biome's id for this specific biosphere. Used to check against itself in the `biome_id_list`
    fn generate_face_chunk(
        &self,
        self_as_dyn: &dyn Biome,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &Noise,
        chunk: &mut Chunk,
        up: BlockFace,
        biome_id_list: &BiomeIdList,
        self_biome_id: u8,
        elevation: &[CoordinateType; (CHUNK_DIMENSIONS * CHUNK_DIMENSIONS) as usize],
        sea_level: Option<&(CoordinateType, Block)>,
    ) {
        generate_face_chunk::<Chunk>(
            self_as_dyn,
            block_coords,
            structure_coords,
            s_dimensions,
            noise_generator,
            chunk,
            up,
            1,
            biome_id_list,
            self_biome_id,
            elevation,
            sea_level,
        );
    }

    /// Generates a chunk that is on an edge of the planet
    /// - `self_as_dyn` Self as a dyn Biome pointer
    /// - `block_coords` The bottom-left-back-most coordinates to start the generation
    /// - `structure_coords` Just used to seed the noise function
    /// - `chunk` The chunk to fill in
    /// - `j_up` The up direction of one of the faces
    /// - `k_up` The up direction of the other of the faces
    /// - `biome_id_list` A list of biomes each block in the chunk is
    /// - `self_biome_id` This biome's id for this specific biosphere. Used to check against itself in the `biome_id_list`
    fn generate_edge_chunk(
        &self,
        self_as_dyn: &dyn Biome,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &Noise,
        chunk: &mut Chunk,
        j_up: BlockFace,
        k_up: BlockFace,
        biome_id_list: &BiomeIdList,
        self_biome_id: u8,
        elevation: &[CoordinateType; CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * 2],
        sea_level: Option<&(CoordinateType, Block)>,
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
            1,
            biome_id_list,
            self_biome_id,
            elevation,
            sea_level,
        );
    }

    /// Generates a chunk that is on a corner of the planet
    /// - `self_as_dyn` Self as a dyn Biome pointer
    /// - `block_coords` The bottom-left-back-most coordinates to start the generation
    /// - `structure_coords` Just used to seed the noise function
    /// - `chunk` The chunk to fill in
    /// - `x_up` The up direction of the x face
    /// - `y_up` The up direction of the y face
    /// - `z_up` The up direction of the z face
    /// - `biome_id_list` A list of biomes each block in the chunk is
    /// - `self_biome_id` This biome's id for this specific biosphere. Used to check against itself in the `biome_id_list`
    fn generate_corner_chunk(
        &self,
        self_as_dyn: &dyn Biome,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &Noise,
        chunk: &mut Chunk,
        x_up: BlockFace,
        y_up: BlockFace,
        z_up: BlockFace,
        biome_id_list: &BiomeIdList,
        self_biome_id: u8,
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
            1,
            biome_id_list,
            self_biome_id,
        );
    }

    /// Generates any features this chunk may have after generating the bulk of the terrain
    ///
    /// This includes things like trees + flora.
    ///
    /// Note that currently LOD chunks will not have this method called.
    fn generate_chunk_features(
        &self,
        block_event_writer: &mut EventWriter<BlockChangedEvent>,
        chunk_coords: ChunkCoordinate,
        structure: &mut Structure,
        structure_location: &Location,
        blocks: &Registry<Block>,
        noise_generator: &Noise,
        seed: &ServerSeed,
    );
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

const LOOKUP_TABLE_PRECISION: usize = 101;
const LOOKUP_TABLE_SIZE: usize = LOOKUP_TABLE_PRECISION * LOOKUP_TABLE_PRECISION * LOOKUP_TABLE_PRECISION;

#[derive(Resource, Clone)]
/// Links a biosphere and all the biomes it has together
///
/// `T` is the marker component for the biosphere this goes with
pub struct BiosphereBiomesRegistry<T: BiosphereMarkerComponent> {
    _phantom: PhantomData<T>,

    /// Contains a list of indicies to the biomes vec
    lookup_table: Arc<RwLock<[u8; LOOKUP_TABLE_SIZE]>>,

    /// All the registered biomes
    biomes: Vec<Arc<RwLock<Box<dyn Biome>>>>,
    /// Only used before `construct_lookup_table` method is called, used to store the biomes + their [`BiomeParameters`] before all the possibilities are computed.
    todo_biomes: Vec<(Vec3, usize)>,
}

#[derive(Clone, Copy, Debug)]
/// Dictates the optimal parameters for this biome to generate.
///
/// The most fit biome will be selected for each block on a planet
pub struct BiomeParameters {
    /// This must be within 0.0 to 100.0
    pub ideal_temperature: f32,
    /// This must be within 0.0 to 100.0
    pub ideal_elevation: f32,
    /// This must be within 0.0 to 100.0
    pub ideal_humidity: f32,
}

impl<T: BiosphereMarkerComponent> Default for BiosphereBiomesRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: BiosphereMarkerComponent> BiosphereBiomesRegistry<T> {
    /// Creates an empty biosphere-biome registry.
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

        let mut lookup_table = self.lookup_table.write().unwrap();

        for here_elevation in 0..LOOKUP_TABLE_PRECISION {
            for here_humidity in 0..LOOKUP_TABLE_PRECISION {
                for here_temperature in 0..LOOKUP_TABLE_PRECISION {
                    let mut best_biome: Option<(f32, usize)> = None;

                    let pos = Vec3::new(here_elevation as f32, here_humidity as f32, here_temperature as f32);

                    for &(params, idx) in self.todo_biomes.iter() {
                        let dist = pos.distance_squared(params);

                        if best_biome.map(|best_b| dist < best_b.0).unwrap_or(true) {
                            best_biome = Some((dist, idx));
                        }
                    }

                    let Some(best_biome) = best_biome else {
                        panic!("Biome registry has no biomes - every biosphere must have at least one biome attached!");
                    };

                    lookup_table[flatten(
                        here_elevation,
                        here_humidity,
                        here_temperature,
                        LOOKUP_TABLE_PRECISION,
                        LOOKUP_TABLE_PRECISION,
                    )] = best_biome.1 as u8;
                }
            }
        }

        info!("Done constructing lookup table!");
    }

    /// Links a biome with this biosphere. Make sure this is only done before `GameState::PostLoading` ends, otherwise this will have no effect.
    pub fn register(&mut self, biome: Arc<RwLock<Box<dyn Biome>>>, params: BiomeParameters) {
        let idx = self.biomes.len();
        self.biomes.push(biome);
        self.todo_biomes.push((
            Vec3::new(params.ideal_elevation, params.ideal_humidity, params.ideal_temperature),
            idx,
        ));
    }

    /// Gets the ideal biome for the parmaters provided
    ///
    /// # Panics
    /// If the params values are outside the range of `[0.0, 100)`, if there was an error getting the RwLock, or if [`construct_lookup_table`] wasn't called yet (run before [`GameState::PostLoading`]` ends)
    pub fn ideal_biome_index_for(&self, params: BiomeParameters) -> usize {
        debug_assert!(
            params.ideal_elevation >= 0.0 && params.ideal_elevation <= 100.0,
            "Bad elevation: {}",
            params.ideal_elevation
        );
        debug_assert!(
            params.ideal_humidity >= 0.0 && params.ideal_humidity <= 100.0,
            "Bad humidity: {}",
            params.ideal_humidity
        );
        debug_assert!(
            params.ideal_temperature >= 0.0 && params.ideal_temperature <= 100.0,
            "Bad temperature: {}",
            params.ideal_temperature
        );

        let lookup_idx = flatten(
            params.ideal_elevation as usize,
            params.ideal_humidity as usize,
            params.ideal_temperature as usize,
            LOOKUP_TABLE_PRECISION,
            LOOKUP_TABLE_PRECISION,
        );

        self.lookup_table.read().unwrap()[lookup_idx] as usize
    }

    #[inline]
    /// Gets the biome from this index (relative to this registry). Call [`Self::ideal_biome_index_for`] to get the best index for a biome.
    pub fn biome_from_index(&self, biome_idx: usize) -> RwLockReadGuard<Box<dyn Biome>> {
        self.biomes[biome_idx].read().unwrap()
    }

    /// Gets the ideal biome for the parmaters provided
    ///
    /// # Panics
    /// If the params values are outside the range of `[0.0, 100)`, if there was an error getting the RwLock, or if [`construct_lookup_table`] wasn't called yet (run before [`GameState::PostLoading`]` ends)
    pub fn ideal_biome_for(&self, params: BiomeParameters) -> RwLockReadGuard<Box<dyn Biome>> {
        let lookup_idx = self.ideal_biome_index_for(params);

        self.biome_from_index(lookup_idx)
    }
}

fn construct_lookup_tables<T: BiosphereMarkerComponent>(mut registry: ResMut<BiosphereBiomesRegistry<T>>) {
    registry.construct_lookup_table();
}

/// This will setup the biosphere registry and construct the lookup tables at the end of [`GameState::PostLoading`]
///
/// You don't normally have to call this manually, because is automatically called in `register_biosphere`
pub fn create_biosphere_biomes_registry<T: BiosphereMarkerComponent>(app: &mut App) {
    app.init_resource::<BiosphereBiomesRegistry<T>>()
        .add_systems(OnExit(GameState::PostLoading), construct_lookup_tables::<T>);
}

pub(super) fn register(app: &mut App) {
    biome_registry::register(app);
    desert::register(app);
    plains::register(app);
    ocean::register(app);
}
