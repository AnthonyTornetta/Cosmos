//! Creates a grass planet
use std::{collections::HashSet, mem::swap};

use bevy::{
    prelude::{
        App, Component, Entity, EventReader, EventWriter, IntoSystemConfigs, OnUpdate, Query, Res,
        ResMut,
    },
    tasks::AsyncComputeTaskPool,
};
use cosmos_core::{
    block::{Block, BlockFace},
    physics::location::Location,
    registry::Registry,
    structure::{
        chunk::{Chunk, CHUNK_DIMENSIONS},
        planet::Planet,
        ChunkInitEvent, Structure,
    },
    utils::{resource_wrapper::ResourceWrapper, timer::UtilsTimer},
};
use futures_lite::future;
use noise::NoiseFn;

use crate::GameState;

use super::{
    register_biosphere, GeneratingChunk, GeneratingChunks, TBiosphere, TGenerateChunkEvent,
    TemperatureRange,
};

#[derive(Component, Debug, Default)]
/// Marks that this is for a grass biosphere
pub struct GrassBiosphereMarker;

/// Marks that a grass chunk needs generated
pub struct GrassChunkNeedsGeneratedEvent {
    x: usize,
    y: usize,
    z: usize,
    structure_entity: Entity,
}

impl TGenerateChunkEvent for GrassChunkNeedsGeneratedEvent {
    fn new(x: usize, y: usize, z: usize, structure_entity: Entity) -> Self {
        Self {
            x,
            y,
            z,
            structure_entity,
        }
    }
}

#[derive(Default, Debug)]
/// Creates a grass planet
pub struct GrassBiosphere;

impl TBiosphere<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent> for GrassBiosphere {
    fn get_marker_component(&self) -> GrassBiosphereMarker {
        GrassBiosphereMarker {}
    }

    fn get_generate_chunk_event(
        &self,
        x: usize,
        y: usize,
        z: usize,
        structure_entity: Entity,
    ) -> GrassChunkNeedsGeneratedEvent {
        GrassChunkNeedsGeneratedEvent::new(x, y, z, structure_entity)
    }
}

const AMPLITUDE: f64 = 7.0;
const DELTA: f64 = 0.05;
const ITERATIONS: usize = 9;

const STONE_LIMIT: usize = 4;

// -y, high z, low positive x not flattening?
// Within (flattening_fraction * planet size) of the 45 starts the flattening.
const FLAT_FRACTION: f64 = 0.4;

// This fraction of the original depth always remains, even on the very edge of the world.
const UNFLATTENED: f64 = 0.25;

fn get_grass_height(
    (mut x, mut y, mut z): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    s_dimensions: usize,
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
) -> usize {
    let mut depth: f64 = 0.0;
    for iteration in 1..=ITERATIONS {
        let iteration = iteration as f64;
        depth += noise_generator.get([
            (x as f64 + structure_x) * (DELTA / iteration),
            (y as f64 + structure_y) * (DELTA / iteration),
            (z as f64 + structure_z) * (DELTA / iteration),
        ]) * AMPLITUDE
            * iteration;
    }

    // For the flattening (it's like the rumbling).
    x = x.min(s_dimensions - x);
    y = y.min(s_dimensions - y);
    z = z.min(s_dimensions - z);

    let initial_height = middle_air_start as f64 + depth;

    // Min is height of the face you're on, second min is the closer to the 45 of the 2 remaining.
    let dist_from_space = s_dimensions as f64 - initial_height;
    let dist_from_45 = x.min(y).max(x.max(y).min(z)) as f64 - dist_from_space;
    let flattening_limit = (s_dimensions as f64 - 2.0 * dist_from_space) * FLAT_FRACTION;
    depth *=
        dist_from_45.min(flattening_limit) / flattening_limit * (1.0 - UNFLATTENED) + UNFLATTENED;

    (middle_air_start as f64 + depth).round() as usize
}

fn notify_when_done_generating(
    mut generating: ResMut<GeneratingChunks<GrassBiosphereMarker>>,
    mut event_writer: EventWriter<ChunkInitEvent>,
    mut structure_query: Query<&mut Structure>,
) {
    let mut still_todo = Vec::with_capacity(generating.generating.len());

    swap(&mut generating.generating, &mut still_todo);

    for mut gg in still_todo {
        if let Some(chunks) = future::block_on(future::poll_once(&mut gg.task)) {
            let (chunk, structure_entity) = chunks;

            if let Ok(mut structure) = structure_query.get_mut(structure_entity) {
                let (x, y, z) = (
                    chunk.structure_x(),
                    chunk.structure_y(),
                    chunk.structure_z(),
                );

                structure.set_chunk(chunk);

                event_writer.send(ChunkInitEvent {
                    structure_entity,
                    x,
                    y,
                    z,
                });
            }
        } else {
            generating.generating.push(gg);
        }
    }
}

#[inline]
fn do_face(
    (sx, sy, sz): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    s_dimensions: usize,
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    grass: &Block,
    dirt: &Block,
    stone: &Block,
    chunk: &mut Chunk,
    up: BlockFace,
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

            let grass_height = get_grass_height(
                seed_coordinates,
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                middle_air_start,
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

                if actual_height < grass_height - STONE_LIMIT {
                    chunk.set_block_at(x, y, z, stone, up);
                } else if actual_height < grass_height {
                    chunk.set_block_at(x, y, z, dirt, up);
                } else if actual_height == grass_height {
                    chunk.set_block_at(x, y, z, grass, up);
                }
            }
        }
    }
}

fn do_edge(
    (sx, sy, sz): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    s_dimensions: usize,
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    grass: &Block,
    dirt: &Block,
    stone: &Block,
    chunk: &mut Chunk,
    j_up: BlockFace,
    k_up: BlockFace,
) {
    let mut j_grass = [[0; CHUNK_DIMENSIONS]; CHUNK_DIMENSIONS];
    for i in 0..CHUNK_DIMENSIONS {
        for k in 0..CHUNK_DIMENSIONS {
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

            // Height of the 45.
            let dim_45 = match k_up {
                BlockFace::Front => z,
                BlockFace::Back => s_dimensions - z,
                BlockFace::Left => s_dimensions - x,
                BlockFace::Right => x,
                BlockFace::Top => y,
                BlockFace::Bottom => s_dimensions - y,
            };

            // Unmodified grass height.
            j_grass[i][k] = get_grass_height(
                (x, y, z),
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                middle_air_start,
            );

            // Don't let the grass fall "below" the 45.
            j_grass[i][k] = j_grass[i][k].max(dim_45);
        }
    }

    for i in 0..CHUNK_DIMENSIONS {
        // The minimum (j, j) on the 45 where the two grass heights intersect.
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

            // Unmodified grass height.
            let mut k_grass = get_grass_height(
                (x, y, z),
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                middle_air_start,
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

            // Don't let the grass fall "below" the 45, but also don't let it go "above" the first shared 45.
            // This probably won't interfere with anything before the first shared 45 is discovered bc of the loop order.
            k_grass = k_grass.clamp(j_height, first_both_45);

            // Get smallest grass height that's on the 45 for both y and z.
            if j_grass[i][j] == j && k_grass == j && first_both_45 == s_dimensions {
                first_both_45 = k_grass;
            };

            for k in 0..CHUNK_DIMENSIONS {
                // Don't let the grass rise "above" the first shared 45.
                let j_grass = j_grass[i][k].min(first_both_45);

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

                let block_up = Planet::planet_face_without_structure(
                    sx + x,
                    sy + y,
                    sz + z,
                    s_dimensions,
                    s_dimensions,
                    s_dimensions,
                );

                // Second height, and also the height of the other 45 (dim_45 in the upper loop must be recalculated here).
                let k_height = match k_up {
                    BlockFace::Front => sz + z,
                    BlockFace::Back => s_dimensions - (sz + z),
                    BlockFace::Left => s_dimensions - (sx + x),
                    BlockFace::Right => sx + x,
                    BlockFace::Top => sy + y,
                    BlockFace::Bottom => s_dimensions - (sy + y),
                };

                if j_height < j_grass - STONE_LIMIT && k_height < k_grass - STONE_LIMIT {
                    chunk.set_block_at(x, y, z, stone, block_up);
                } else if j_height < j_grass && k_height < k_grass {
                    chunk.set_block_at(x, y, z, dirt, block_up);
                } else if j_height == j_grass && k_height < k_grass {
                    chunk.set_block_at(x, y, z, grass, j_up);
                } else if j_height < j_grass && k_height == k_grass {
                    chunk.set_block_at(x, y, z, grass, k_up);
                }
            }
        }
    }
}

fn do_corner(
    (sx, sy, sz): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    s_dimensions: usize,
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    grass: &Block,
    dirt: &Block,
    stone: &Block,
    chunk: &mut Chunk,
    x_up: BlockFace,
    y_up: BlockFace,
    z_up: BlockFace,
) {
    // x grass height cache.
    let mut x_grass = [[0; CHUNK_DIMENSIONS]; CHUNK_DIMENSIONS];
    for j in 0..CHUNK_DIMENSIONS {
        for k in 0..CHUNK_DIMENSIONS {
            // Seed coordinates for the noise function.
            let (x, y, z) = match x_up {
                BlockFace::Right => (middle_air_start, sy + j, sz + k),
                _ => (s_dimensions - middle_air_start, sy + j, sz + k),
            };

            // Unmodified grass height.
            x_grass[j][k] = get_grass_height(
                (x, y, z),
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                middle_air_start,
            );

            // Don't let the grass fall "below" the 45s.
            let y_45 = match y_up {
                BlockFace::Top => y,
                _ => s_dimensions - y,
            };
            let z_45 = match z_up {
                BlockFace::Front => z,
                _ => s_dimensions - z,
            };
            x_grass[j][k] = x_grass[j][k].max(y_45).max(z_45);
        }
    }

    // y grass height cache.
    let mut y_grass = [[0; CHUNK_DIMENSIONS]; CHUNK_DIMENSIONS];
    for i in 0..CHUNK_DIMENSIONS {
        for k in 0..CHUNK_DIMENSIONS {
            // Seed coordinates for the noise function. Which loop variable goes to which xyz must agree everywhere.
            let (x, y, z) = match y_up {
                BlockFace::Top => (sx + i, middle_air_start, sz + k),
                _ => (sx + i, s_dimensions - middle_air_start, sz + k),
            };

            // Unmodified grass height.
            y_grass[i][k] = get_grass_height(
                (x, y, z),
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                middle_air_start,
            );

            // Don't let the grass fall "below" the 45s.
            let x_45 = match x_up {
                BlockFace::Right => x,
                _ => s_dimensions - x,
            };
            let z_45 = match z_up {
                BlockFace::Front => z,
                _ => s_dimensions - z,
            };
            y_grass[i][k] = y_grass[i][k].max(x_45).max(z_45);
        }
    }

    for i in 0..CHUNK_DIMENSIONS {
        // The minimum (j, j, j) on the 45 where the three grass heights intersect.
        let mut first_all_45 = s_dimensions;
        for j in 0..CHUNK_DIMENSIONS {
            // Seed coordinates for the noise function.
            let (x, y, z) = match z_up {
                BlockFace::Front => (sx + i, sy + j, middle_air_start),
                _ => (sx + i, sy + j, s_dimensions - middle_air_start),
            };

            // Unmodified grass height.
            let mut z_grass = get_grass_height(
                (x, y, z),
                (structure_x, structure_y, structure_z),
                s_dimensions,
                noise_generator,
                middle_air_start,
            );

            let x_height = match x_up {
                BlockFace::Right => x,
                _ => s_dimensions - x,
            };

            let y_height = match y_up {
                BlockFace::Top => y,
                _ => s_dimensions - y,
            };

            // Don't let the grass fall "below" the 45, but also don't let it go "above" the first shared 45.
            // This probably won't interfere with anything before the first shared 45 is discovered bc of the loop order.
            z_grass = z_grass.max(x_height).max(y_height);
            z_grass = z_grass.min(first_all_45);

            // Get smallest grass height that's on the 45 for x, y, and z.
            if x_grass[i][j] == j
                && y_grass[i][j] == j
                && z_grass == j
                && first_all_45 == s_dimensions
            {
                first_all_45 = z_grass;
            };

            for k in 0..CHUNK_DIMENSIONS {
                // Don't let the grass rise "above" the first shared 45.
                let x_grass = x_grass[j][k].min(first_all_45);
                let y_grass = y_grass[i][k].min(first_all_45);

                let z = sz + k;
                let block_up = Planet::planet_face_without_structure(
                    x,
                    y,
                    z,
                    s_dimensions,
                    s_dimensions,
                    s_dimensions,
                );

                let z_height = match z_up {
                    BlockFace::Front => z,
                    _ => s_dimensions - z,
                };

                if x_height < x_grass - STONE_LIMIT
                    && y_height < y_grass - STONE_LIMIT
                    && z_height < z_grass - STONE_LIMIT
                {
                    chunk.set_block_at(i, j, k, stone, block_up);
                } else if x_height < x_grass && y_height < y_grass && z_height < z_grass {
                    chunk.set_block_at(i, j, k, dirt, block_up);
                } else if x_height == x_grass && y_height < y_grass && z_height < z_grass {
                    chunk.set_block_at(i, j, k, grass, x_up);
                } else if x_height < x_grass && y_height == y_grass && z_height < z_grass {
                    chunk.set_block_at(i, j, k, grass, y_up);
                } else if x_height < x_grass && y_height < y_grass && z_height == z_grass {
                    chunk.set_block_at(i, j, k, grass, z_up);
                }
            }
        }
    }
}

fn generate_planet(
    mut query: Query<(&mut Structure, &Location)>,
    mut generating: ResMut<GeneratingChunks<GrassBiosphereMarker>>,
    mut events: EventReader<GrassChunkNeedsGeneratedEvent>,
    noise_generator: Res<ResourceWrapper<noise::OpenSimplex>>,
    blocks: Res<Registry<Block>>,
) {
    let chunks = events
        .iter()
        .filter_map(|ev| {
            if let Ok((mut structure, _)) = query.get_mut(ev.structure_entity) {
                Some((
                    ev.structure_entity,
                    structure.take_or_create_chunk_for_loading(ev.x, ev.y, ev.z),
                ))
            } else {
                None
            }
        })
        .collect::<Vec<(Entity, Chunk)>>();

    let grass = blocks.from_id("cosmos:grass").unwrap();
    let dirt = blocks.from_id("cosmos:dirt").unwrap();
    let stone = blocks.from_id("cosmos:stone").unwrap();

    let thread_pool = AsyncComputeTaskPool::get();

    let chunks = chunks
        .into_iter()
        .flat_map(|(structure_entity, chunk)| {
            let Ok((structure, location)) = query.get(structure_entity) else {
                return None;
            };

            let s_width = structure.blocks_width();
            let s_height = structure.blocks_height();
            let s_length = structure.blocks_length();
            let location = *location;

            Some((
                chunk,
                s_width,
                s_height,
                s_length,
                location,
                structure_entity,
            ))
        })
        .collect::<Vec<(Chunk, usize, usize, usize, Location, Entity)>>();

    if !chunks.is_empty() {
        println!("Doing {} chunks!", chunks.len());

        for (mut chunk, s_width, s_height, s_length, location, structure_entity) in chunks {
            let grass = grass.clone();
            let dirt = dirt.clone();
            let stone = stone.clone();
            // Not super expensive, only copies about 256 8 bit values.
            // Still not ideal though.
            let noise_generator = **noise_generator;

            let task = thread_pool.spawn(async move {
                let timer = UtilsTimer::start();
                let grass = &grass;
                let dirt = &dirt;
                let stone = &stone;

                let middle_air_start = s_height - CHUNK_DIMENSIONS * 5;

                let actual_pos = location.absolute_coords_f64();

                let structure_z = actual_pos.z;
                let structure_y = actual_pos.y;
                let structure_x = actual_pos.x;

                // To save multiplication operations later.
                let sz = chunk.structure_z() * CHUNK_DIMENSIONS;
                let sy = chunk.structure_y() * CHUNK_DIMENSIONS;
                let sx = chunk.structure_x() * CHUNK_DIMENSIONS;

                // Get all possible planet faces from the chunk corners. May or may not break near the center of the planet.
                let mut planet_faces = HashSet::new();
                for z in 0..=1 {
                    for y in 0..=1 {
                        for x in 0..=1 {
                            planet_faces.insert(Planet::planet_face_without_structure(
                                sx + x * CHUNK_DIMENSIONS,
                                sy + y * CHUNK_DIMENSIONS,
                                sz + z * CHUNK_DIMENSIONS,
                                s_width,
                                s_height,
                                s_length,
                            ));
                        }
                    }
                }

                // Support for the middle of the planet.
                if planet_faces.contains(&BlockFace::Top) {
                    planet_faces.remove(&BlockFace::Bottom);
                }
                if planet_faces.contains(&BlockFace::Right) {
                    planet_faces.remove(&BlockFace::Left);
                }
                if planet_faces.contains(&BlockFace::Front) {
                    planet_faces.remove(&BlockFace::Back);
                }

                if planet_faces.len() == 1 {
                    // Chunks on only one face.
                    do_face(
                        (sx, sy, sz),
                        (structure_x, structure_y, structure_z),
                        s_height,
                        &noise_generator,
                        middle_air_start,
                        grass,
                        dirt,
                        stone,
                        &mut chunk,
                        *planet_faces.iter().next().unwrap(),
                    );
                } else if planet_faces.len() == 2 {
                    // Chunks on an edge.
                    let mut face_iter = planet_faces.iter();
                    do_edge(
                        (sx, sy, sz),
                        (structure_x, structure_y, structure_z),
                        s_height,
                        &noise_generator,
                        middle_air_start,
                        grass,
                        dirt,
                        stone,
                        &mut chunk,
                        *face_iter.next().unwrap(),
                        *face_iter.next().unwrap(),
                    );
                } else {
                    let x_face = if planet_faces.contains(&BlockFace::Right) {
                        BlockFace::Right
                    } else {
                        BlockFace::Left
                    };
                    let y_face = if planet_faces.contains(&BlockFace::Top) {
                        BlockFace::Top
                    } else {
                        BlockFace::Bottom
                    };
                    let z_face = if planet_faces.contains(&BlockFace::Front) {
                        BlockFace::Front
                    } else {
                        BlockFace::Back
                    };
                    do_corner(
                        (sx, sy, sz),
                        (structure_x, structure_y, structure_z),
                        s_height,
                        &noise_generator,
                        middle_air_start,
                        grass,
                        dirt,
                        stone,
                        &mut chunk,
                        x_face,
                        y_face,
                        z_face,
                    );
                }
                timer.log_duration("Chunk: ");
                (chunk, structure_entity)
            });

            generating.generating.push(GeneratingChunk::new(task));
        }
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent>(
        app,
        "cosmos:biosphere_grass",
        TemperatureRange::new(0.0, 1000000000.0),
    );

    app.add_systems(
        (generate_planet, notify_when_done_generating).in_set(OnUpdate(GameState::Playing)),
    );
}
