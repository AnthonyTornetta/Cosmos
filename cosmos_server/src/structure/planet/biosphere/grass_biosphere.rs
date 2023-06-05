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
    block::{blocks::AIR_BLOCK_ID, Block, BlockFace},
    physics::location::Location,
    registry::{identifiable::Identifiable, Registry},
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

fn get_max_level(
    (x, y, z): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
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
    (middle_air_start as f64 + depth).round() as usize
}

#[inline]
fn generate_block(
    (x, y, z): (usize, usize, usize),
    (actual_x, actual_y, actual_z): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    (s_width, s_height, s_length): (usize, usize, usize),
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    grass: &Block,
    dirt: &Block,
    stone: &Block,
    chunk: &mut Chunk,
) {
    let current_max = get_max_level(
        (actual_x, actual_y, actual_z),
        (structure_x, structure_y, structure_z),
        noise_generator,
        middle_air_start,
    );

    // Consider not doing this unless the cover data is needed.
    let mut cover_x = actual_x as i64;
    let mut cover_y = actual_y as i64;
    let mut cover_z = actual_z as i64;
    let block_up = Planet::planet_face_without_structure(
        actual_x, actual_y, actual_z, s_width, s_height, s_length,
    );

    let current_height = match block_up {
        BlockFace::Top => {
            cover_y += 1;
            actual_y
        }
        BlockFace::Bottom => {
            cover_y -= 1;
            s_height - actual_y
        }
        BlockFace::Front => {
            cover_z += 1;
            actual_z
        }
        BlockFace::Back => {
            cover_z -= 1;
            s_height - actual_z
        }
        BlockFace::Right => {
            cover_x += 1;
            actual_x
        }
        BlockFace::Left => {
            cover_x -= 1;
            s_height - actual_x
        }
    };

    if current_height < current_max - STONE_LIMIT {
        chunk.set_block_at(x, y, z, stone, block_up);
    } else if current_height < current_max {
        // Getting the noise values for the "covering" block.
        let cover_height = current_height + 1;

        let cover_max = if cover_x < 0 || cover_y < 0 || cover_z < 0 {
            0
        } else {
            get_max_level(
                (cover_x as usize, cover_y as usize, cover_z as usize),
                (structure_x, structure_y, structure_z),
                noise_generator,
                middle_air_start,
            )
        };

        if cover_height < cover_max {
            // In dirt range and covered -> dirt.
            chunk.set_block_at(x, y, z, dirt, block_up)
        } else {
            // In dirt range and uncovered -> grass.
            chunk.set_block_at(x, y, z, grass, block_up)
        }
    }
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
    (s_width, s_height, s_length): (usize, usize, usize),
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
                BlockFace::Bottom => (sx + i, s_height - middle_air_start, sz + j),
                BlockFace::Front => (sx + i, sy + j, middle_air_start),
                BlockFace::Back => (sx + i, sy + j, s_length - middle_air_start),
                BlockFace::Right => (middle_air_start, sy + i, sz + j),
                BlockFace::Left => (s_width - middle_air_start, sy + i, sz + j),
            };

            let grass_height = get_max_level(
                seed_coordinates,
                (structure_x, structure_y, structure_z),
                noise_generator,
                middle_air_start,
            );

            for height in 0..CHUNK_DIMENSIONS {
                let (x, y, z, actual_height) = match up {
                    BlockFace::Top => (i, height, j, sy + height),
                    BlockFace::Bottom => (i, height, j, s_height - (sy + height)),
                    BlockFace::Front => (i, j, height, sz + height),
                    BlockFace::Back => (i, j, height, s_length - (sz + height)),
                    BlockFace::Right => (height, i, j, sx + height),
                    BlockFace::Left => (height, i, j, s_width - (sx + height)),
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

fn do_top_front_edge(
    (sx, sy, sz): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    (s_width, s_height, s_length): (usize, usize, usize),
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    grass: &Block,
    dirt: &Block,
    stone: &Block,
    chunk: &mut Chunk,
) {
    let mut y_grass = [[0; CHUNK_DIMENSIONS]; CHUNK_DIMENSIONS];
    for i in 0..CHUNK_DIMENSIONS {
        for k in 0..CHUNK_DIMENSIONS {
            let (x, y, z) = (sx + i, middle_air_start, sz + k);
            y_grass[i][k] = get_max_level(
                (x, y, z),
                (structure_x, structure_y, structure_z),
                noise_generator,
                middle_air_start,
            );
            y_grass[i][k] = y_grass[i][k].max(z);
        }
    }

    let mut z_grass = [[0; CHUNK_DIMENSIONS]; CHUNK_DIMENSIONS];
    for i in 0..CHUNK_DIMENSIONS {
        for j in 0..CHUNK_DIMENSIONS {
            let (x, y, z) = (sx + i, sy + j, middle_air_start);
            z_grass[i][j] = get_max_level(
                (x, y, z),
                (structure_x, structure_y, structure_z),
                noise_generator,
                middle_air_start,
            );
            z_grass[i][j] = z_grass[i][j].max(y);
        }
    }

    for i in 0..CHUNK_DIMENSIONS {
        // Get smallest grass height that's on the 45 for both y and z.
        let mut min_45 = 0;
        while min_45 < CHUNK_DIMENSIONS
            && !(y_grass[i][min_45] == min_45 && z_grass[i][min_45] == min_45)
        {
            min_45 += 1;
        }

        // Cap the grass height at the smallest 45 for every block that comes after.
        for j in (min_45 - sy)..CHUNK_DIMENSIONS {
            z_grass[i][j] = z_grass[i][j].min(min_45);
        }

        for k in (min_45 - sz)..CHUNK_DIMENSIONS {
            y_grass[i][k] = y_grass[i][k].min(min_45);
        }

        for j in 0..CHUNK_DIMENSIONS {
            let height1 = sy + j;
            for k in 0..CHUNK_DIMENSIONS {
                let (x, y, z, height2) = (i, j, k, sz + k);
                let block_up = Planet::planet_face_without_structure(
                    sx + x,
                    sy + y,
                    sz + z,
                    s_width,
                    s_height,
                    s_length,
                );

                // if height1 == y_grass[i][k] {
                //     chunk.set_block_at(x, y, z, stone, block_up)
                // }

                // if height2 == z_grass[i][j] {
                //     chunk.set_block_at(x, y, z, dirt, block_up)
                // }

                if height1 < y_grass[i][k] - STONE_LIMIT && height2 < z_grass[i][j] - STONE_LIMIT {
                    chunk.set_block_at(x, y, z, stone, block_up);
                } else if height1 < y_grass[i][k] && height2 < z_grass[i][j] {
                    chunk.set_block_at(x, y, z, dirt, block_up);
                } else if height1 < y_grass[i][k] && height2 == z_grass[i][j] {
                    chunk.set_block_at(x, y, z, grass, BlockFace::Front);
                } else if height1 == y_grass[i][k] && height2 < z_grass[i][j] {
                    chunk.set_block_at(x, y, z, grass, BlockFace::Top);
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

                if planet_faces.len() == 1 {
                    // Chunks on only one face.
                    do_face(
                        (sx, sy, sz),
                        (structure_x, structure_y, structure_z),
                        (s_width, s_height, s_length),
                        &noise_generator,
                        middle_air_start,
                        grass,
                        dirt,
                        stone,
                        &mut chunk,
                        *planet_faces.iter().next().unwrap(),
                    );
                } else if planet_faces.len() == 2 {
                    do_top_front_edge(
                        (sx, sy, sz),
                        (structure_x, structure_y, structure_z),
                        (s_width, s_height, s_length),
                        &noise_generator,
                        middle_air_start,
                        grass,
                        dirt,
                        stone,
                        &mut chunk,
                    );
                } else {
                    for z in 0..CHUNK_DIMENSIONS {
                        let actual_z = sz + z;
                        for y in 0..CHUNK_DIMENSIONS {
                            let actual_y: usize = sy + y;
                            for x in 0..CHUNK_DIMENSIONS {
                                if chunk.has_block_at(x, y, z) {
                                    continue;
                                }

                                let actual_x = sx + x;
                                generate_block(
                                    (x, y, z),
                                    (actual_x, actual_y, actual_z),
                                    (structure_x, structure_y, structure_z),
                                    (s_width, s_height, s_length),
                                    &noise_generator,
                                    middle_air_start,
                                    grass,
                                    dirt,
                                    stone,
                                    &mut chunk,
                                );
                            }
                        }
                    }
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
