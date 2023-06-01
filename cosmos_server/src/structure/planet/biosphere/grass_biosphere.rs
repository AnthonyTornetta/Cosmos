//! Creates a grass planet
use std::mem::swap;

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
fn do_top_face(
    (x, mut y_up, z): (usize, usize, usize),
    (actual_x, sy, actual_z): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    (s_width, s_height, s_length): (usize, usize, usize),
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    grass: &Block,
    dirt: &Block,
    stone: &Block,
    chunk: &mut Chunk,
) -> usize {
    let mut air_start = get_max_level(
        (actual_x, sy + y_up, actual_z),
        (structure_x, structure_y, structure_z),
        noise_generator,
        middle_air_start,
    );

    // While in a solid block, move up a step.
    while y_up < CHUNK_DIMENSIONS && sy + y_up < air_start {
        y_up += 1;
        air_start = get_max_level(
            (actual_x, sy + y_up, actual_z),
            (structure_x, structure_y, structure_z),
            noise_generator,
            middle_air_start,
        );
    }

    // While in an air block, move down a step.
    while y_up != 0 && sy + y_up >= air_start {
        y_up -= 1;
        air_start = get_max_level(
            (actual_x, sy + y_up, actual_z),
            (structure_x, structure_y, structure_z),
            noise_generator,
            middle_air_start,
        );
    }

    // At this point, we should always be at the top solid block, or at the top or bottom boundary.
    if y_up == 0 {
        // All air column, except for possibly the bottom block.
        generate_block(
            (x, 0, z),
            (actual_x, sy, actual_z),
            (structure_x, structure_y, structure_z),
            (s_width, s_height, s_length),
            noise_generator,
            middle_air_start,
            grass,
            dirt,
            stone,
            chunk,
        );
        1
    } else if y_up >= CHUNK_DIMENSIONS {
        // All solid column, generate the top 4 blocks the old fashion way (there might be a grass block 1 step up in a different chunk).
        let stone_line = CHUNK_DIMENSIONS - STONE_LIMIT;
        for y in stone_line..CHUNK_DIMENSIONS {
            generate_block(
                (x, y, z),
                (actual_x, sy + y, actual_z),
                (structure_x, structure_y, structure_z),
                (s_width, s_height, s_length),
                noise_generator,
                middle_air_start,
                grass,
                dirt,
                stone,
                chunk,
            );
        }

        // All blocks below those 4 are definitely stone.
        for y in 0..stone_line {
            chunk.set_block_at(x, y, z, stone, BlockFace::Top);
        }
        CHUNK_DIMENSIONS - 1
    } else {
        // Mixed air and solid, we are on the dividing line. Set current to grass, next 5 dirt, then stone.
        chunk.set_block_at(x, y_up, z, grass, BlockFace::Top);
        let stone_line = 0.max(y_up as i32 - STONE_LIMIT as i32) as usize;
        for y in 0..stone_line {
            chunk.set_block_at(x, y, z, stone, BlockFace::Top);
        }
        for y in stone_line..y_up {
            chunk.set_block_at(x, y, z, dirt, BlockFace::Top)
        }
        y_up
    }
}

#[inline]
fn do_right_face(
    (mut x_up, y, z): (usize, usize, usize),
    (sx, actual_y, actual_z): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    (s_width, s_height, s_length): (usize, usize, usize),
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    grass: &Block,
    dirt: &Block,
    stone: &Block,
    chunk: &mut Chunk,
) -> usize {
    let mut air_start: usize = get_max_level(
        (sx + x_up, actual_y, actual_z),
        (structure_x, structure_y, structure_z),
        noise_generator,
        middle_air_start,
    );

    // While in a solid block, move up a step.
    while x_up < CHUNK_DIMENSIONS && sx + x_up < air_start {
        x_up += 1;
        air_start = get_max_level(
            (sx + x_up, actual_y, actual_z),
            (structure_x, structure_y, structure_z),
            noise_generator,
            middle_air_start,
        );
    }

    // While in an air block, move down a step.
    while x_up != 0 && sx + x_up >= air_start {
        x_up -= 1;
        air_start = get_max_level(
            (sx + x_up, actual_y, actual_z),
            (structure_x, structure_y, structure_z),
            noise_generator,
            middle_air_start,
        );
    }

    // At this point, we should always be at the top solid block, or at the top or bottom boundary.
    if x_up == 0 {
        // All air column, except for possibly the bottom block.
        generate_block(
            (0, y, z),
            (sx + x_up, actual_y, actual_z),
            (structure_x, structure_y, structure_z),
            (s_width, s_height, s_length),
            noise_generator,
            middle_air_start,
            grass,
            dirt,
            stone,
            chunk,
        );
        1
    } else if x_up >= CHUNK_DIMENSIONS {
        // All solid column, generate the top 4 blocks the old fashion way (there might be a grass block 1 step up in a different chunk).
        let stone_line = CHUNK_DIMENSIONS - STONE_LIMIT;
        for x in stone_line..CHUNK_DIMENSIONS {
            generate_block(
                (x, y, z),
                (sx + x, actual_y, actual_z),
                (structure_x, structure_y, structure_z),
                (s_width, s_height, s_length),
                noise_generator,
                middle_air_start,
                grass,
                dirt,
                stone,
                chunk,
            );
        }

        // All blocks below those 4 are definitely stone.
        for x in 0..stone_line {
            chunk.set_block_at(x, y, z, stone, BlockFace::Right);
        }
        CHUNK_DIMENSIONS - 1
    } else {
        // Mixed air and solid, we are on the dividing line. Set current to grass, next 5 dirt, then stone.
        chunk.set_block_at(x_up, y, z, grass, BlockFace::Right);
        let stone_line = 0.max(x_up as i32 - STONE_LIMIT as i32) as usize;
        for x in 0..stone_line {
            chunk.set_block_at(x, y, z, stone, BlockFace::Right);
        }
        for x in stone_line..x_up {
            chunk.set_block_at(x, y, z, dirt, BlockFace::Right)
        }
        x_up
    }
}

#[inline]
fn do_front_face(
    (x, y, mut z_up): (usize, usize, usize),
    (actual_x, actual_y, sz): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    (s_width, s_height, s_length): (usize, usize, usize),
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    grass: &Block,
    dirt: &Block,
    stone: &Block,
    chunk: &mut Chunk,
) -> usize {
    let mut air_start = get_max_level(
        (actual_x, actual_y, sz + z_up),
        (structure_x, structure_y, structure_z),
        noise_generator,
        middle_air_start,
    );

    // While in a solid block, move up a step.
    while z_up < CHUNK_DIMENSIONS && sz + z_up < air_start {
        z_up += 1;
        air_start = get_max_level(
            (actual_x, actual_y, sz + z_up),
            (structure_x, structure_y, structure_z),
            noise_generator,
            middle_air_start,
        );
    }

    // While in an air block, move down a step.
    while z_up != 0 && sz + z_up >= air_start {
        z_up -= 1;
        air_start = get_max_level(
            (actual_x, actual_y, sz + z_up),
            (structure_x, structure_y, structure_z),
            noise_generator,
            middle_air_start,
        );
    }

    // At this point, we should always be at the top solid block, or at the top or bottom boundary.
    if z_up == 0 {
        // All air column, except for possibly the bottom block.
        generate_block(
            (x, y, 0),
            (actual_x, actual_y, sz),
            (structure_x, structure_y, structure_z),
            (s_width, s_height, s_length),
            noise_generator,
            middle_air_start,
            grass,
            dirt,
            stone,
            chunk,
        );
        1
    } else if z_up >= CHUNK_DIMENSIONS {
        // All solid column, generate the top 4 blocks the old fashion way (there might be a grass block 1 step up in a different chunk).
        let stone_line = CHUNK_DIMENSIONS - STONE_LIMIT;
        for z in stone_line..CHUNK_DIMENSIONS {
            generate_block(
                (x, y, z),
                (actual_x, actual_y, sz + z),
                (structure_x, structure_y, structure_z),
                (s_width, s_height, s_length),
                noise_generator,
                middle_air_start,
                grass,
                dirt,
                stone,
                chunk,
            );
        }

        // All blocks below those 4 are definitely stone.
        for z in 0..stone_line {
            chunk.set_block_at(x, y, z, stone, BlockFace::Front);
        }
        CHUNK_DIMENSIONS - 1
    } else {
        // Mixed air and solid, we are on the dividing line. Set current to grass, next 5 dirt, then stone.
        chunk.set_block_at(x, y, z_up, grass, BlockFace::Front);
        let stone_line = 0.max(z_up as i32 - STONE_LIMIT as i32) as usize;
        for z in 0..stone_line {
            chunk.set_block_at(x, y, z, stone, BlockFace::Front);
        }
        for z in stone_line..z_up {
            chunk.set_block_at(x, y, z, dirt, BlockFace::Front)
        }
        z_up
    }
}

#[inline]
fn do_bottom_face(
    (x, mut y_up, z): (usize, usize, usize),
    (actual_x, sy, actual_z): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    (s_width, s_height, s_length): (usize, usize, usize),
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    grass: &Block,
    dirt: &Block,
    stone: &Block,
    chunk: &mut Chunk,
) -> usize {
    let mut air_start = get_max_level(
        (actual_x, sy + y_up - 1, actual_z),
        (structure_x, structure_y, structure_z),
        noise_generator,
        middle_air_start,
    );

    // While in a solid block, move down a step.
    while y_up >= 1 && s_height - (sy + y_up - 1) < air_start {
        y_up -= 1;
        air_start = get_max_level(
            (actual_x, sy + y_up - 1, actual_z),
            (structure_x, structure_y, structure_z),
            noise_generator,
            middle_air_start,
        );
    }

    // While in an air block, move up a step.
    while y_up <= CHUNK_DIMENSIONS && s_height - (sy + y_up - 1) >= air_start {
        y_up += 1;
        air_start = get_max_level(
            (actual_x, sy + y_up - 1, actual_z),
            (structure_x, structure_y, structure_z),
            noise_generator,
            middle_air_start,
        );
    }

    // At this point, we should always be at the top solid block, or at the top or bottom boundary.
    if y_up == 0 {
        // All solid column, generate the top 4 blocks the old fashion way (there might be a grass block 1 step up in a different chunk).
        for y in 0..STONE_LIMIT {
            generate_block(
                (x, y, z),
                (actual_x, sy + y, actual_z),
                (structure_x, structure_y, structure_z),
                (s_width, s_height, s_length),
                &noise_generator,
                middle_air_start,
                grass,
                dirt,
                stone,
                chunk,
            );
        }

        // All blocks below those 4 are definitely stone.
        for y in STONE_LIMIT..CHUNK_DIMENSIONS {
            chunk.set_block_at(x, y, z, stone, BlockFace::Bottom);
        }
        1
    } else if y_up >= CHUNK_DIMENSIONS {
        // All air column, except for possibly the bottom block.
        generate_block(
            (x, y_up - 1, z),
            (actual_x, sy + y_up - 1, actual_z),
            (structure_x, structure_y, structure_z),
            (s_width, s_height, s_length),
            noise_generator,
            middle_air_start,
            grass,
            dirt,
            stone,
            chunk,
        );
        CHUNK_DIMENSIONS - 1
    } else {
        // Mixed air and solid, we are on the dividing line. Set current to grass, next 5 dirt, then stone.
        chunk.set_block_at(x, y_up - 1, z, grass, BlockFace::Bottom);
        let stone_line = CHUNK_DIMENSIONS.min(y_up - 1 + STONE_LIMIT);
        for y in y_up..stone_line {
            chunk.set_block_at(x, y, z, dirt, BlockFace::Bottom);
        }
        for y in stone_line..CHUNK_DIMENSIONS {
            chunk.set_block_at(x, y, z, stone, BlockFace::Bottom)
        }
        y_up
    }
}

#[inline]
fn do_left_face(
    (mut x_up, y, z): (usize, usize, usize),
    (sx, actual_y, actual_z): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    (s_width, s_height, s_length): (usize, usize, usize),
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    grass: &Block,
    dirt: &Block,
    stone: &Block,
    chunk: &mut Chunk,
) -> usize {
    let mut air_start = get_max_level(
        (sx + x_up - 1, actual_y, actual_z),
        (structure_x, structure_y, structure_z),
        noise_generator,
        middle_air_start,
    );

    // While in a solid block, move down a step.
    while x_up >= 1 && s_height - (sx + x_up - 1) < air_start {
        x_up -= 1;
        air_start = get_max_level(
            (sx + x_up - 1, actual_y, actual_z),
            (structure_x, structure_y, structure_z),
            noise_generator,
            middle_air_start,
        );
    }

    // While in an air block, move up a step.
    while x_up <= CHUNK_DIMENSIONS && s_height - (sx + x_up - 1) >= air_start {
        x_up += 1;
        air_start = get_max_level(
            (sx + x_up - 1, actual_y, actual_z),
            (structure_x, structure_y, structure_z),
            noise_generator,
            middle_air_start,
        );
    }

    // At this point, we should always be at the top solid block, or at the top or bottom boundary.
    if x_up == 0 {
        // All solid column, generate the top 4 blocks the old fashion way (there might be a grass block 1 step up in a different chunk).
        for x in 0..STONE_LIMIT {
            generate_block(
                (x, y, z),
                (sx + x, actual_y, actual_z),
                (structure_x, structure_y, structure_z),
                (s_width, s_height, s_length),
                &noise_generator,
                middle_air_start,
                grass,
                dirt,
                stone,
                chunk,
            );
        }

        // All blocks below those 4 are definitely stone.
        for x in STONE_LIMIT..CHUNK_DIMENSIONS {
            chunk.set_block_at(x, y, z, stone, BlockFace::Left);
        }
        1
    } else if x_up >= CHUNK_DIMENSIONS {
        // All air column, except for possibly the bottom block.
        generate_block(
            (x_up - 1, y, z),
            (sx + x_up - 1, actual_y, actual_z),
            (structure_x, structure_y, structure_z),
            (s_width, s_height, s_length),
            noise_generator,
            middle_air_start,
            grass,
            dirt,
            stone,
            chunk,
        );
        CHUNK_DIMENSIONS - 1
    } else {
        // Mixed air and solid, we are on the dividing line. Set current to grass, next 5 dirt, then stone.
        chunk.set_block_at(x_up, y, z, grass, BlockFace::Left);
        let stone_line = CHUNK_DIMENSIONS.min(x_up - 1 + STONE_LIMIT);
        for x in x_up..stone_line {
            chunk.set_block_at(x, y, z, dirt, BlockFace::Left);
        }
        for x in stone_line..CHUNK_DIMENSIONS {
            chunk.set_block_at(x, y, z, stone, BlockFace::Left)
        }
        x_up
    }
}

#[inline]
fn do_back_face(
    (x, y, mut z_up): (usize, usize, usize),
    (actual_x, actual_y, sz): (usize, usize, usize),
    (structure_x, structure_y, structure_z): (f64, f64, f64),
    (s_width, s_height, s_length): (usize, usize, usize),
    noise_generator: &noise::OpenSimplex,
    middle_air_start: usize,
    grass: &Block,
    dirt: &Block,
    stone: &Block,
    chunk: &mut Chunk,
) -> usize {
    let mut air_start = get_max_level(
        (actual_x, actual_y, sz + z_up - 1),
        (structure_x, structure_y, structure_z),
        noise_generator,
        middle_air_start,
    );

    // While in a solid block, move down a step.
    while z_up >= 1 && s_height - (sz + z_up - 1) < air_start {
        z_up -= 1;
        air_start = get_max_level(
            (actual_x, actual_y, sz + z_up - 1),
            (structure_x, structure_y, structure_z),
            noise_generator,
            middle_air_start,
        );
    }

    // While in an air block, move up a step.
    while z_up <= CHUNK_DIMENSIONS && s_height - (sz + z_up - 1) >= air_start {
        z_up += 1;
        air_start = get_max_level(
            (actual_x, actual_y, sz + z_up - 1),
            (structure_x, structure_y, structure_z),
            noise_generator,
            middle_air_start,
        );
    }

    // At this point, we should always be at the top solid block, or at the top or bottom boundary.
    if z_up == 0 {
        // All solid column, generate the top 4 blocks the old fashion way (there might be a grass block 1 step up in a different chunk).
        for z in 0..STONE_LIMIT {
            generate_block(
                (x, y, z),
                (actual_x, actual_y, sz + z),
                (structure_x, structure_y, structure_z),
                (s_width, s_height, s_length),
                &noise_generator,
                middle_air_start,
                grass,
                dirt,
                stone,
                chunk,
            );
        }

        // All blocks below those 4 are definitely stone.
        for z in STONE_LIMIT..CHUNK_DIMENSIONS {
            chunk.set_block_at(x, y, z, stone, BlockFace::Back);
        }
        1
    } else if z_up >= CHUNK_DIMENSIONS {
        // All air column, except for possibly the bottom block.
        generate_block(
            (x, y, z_up - 1),
            (actual_x, actual_y, sz + z_up - 1),
            (structure_x, structure_y, structure_z),
            (s_width, s_height, s_length),
            noise_generator,
            middle_air_start,
            grass,
            dirt,
            stone,
            chunk,
        );
        CHUNK_DIMENSIONS - 1
    } else {
        // Mixed air and solid, we are on the dividing line. Set current to grass, next 5 dirt, then stone.
        chunk.set_block_at(x, y, z_up - 1, grass, BlockFace::Back);
        let stone_line = CHUNK_DIMENSIONS.min(z_up - 1 + STONE_LIMIT);
        for z in z_up..stone_line {
            chunk.set_block_at(x, y, z, dirt, BlockFace::Back);
        }
        for z in stone_line..CHUNK_DIMENSIONS {
            chunk.set_block_at(x, y, z, stone, BlockFace::Back)
        }
        z_up
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
                // let timer = UtilsTimer::start();
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

                // Get all possible planet faces from the chunk corners (max 3).
                // May or may not break near the center of the planet, but that's all stone for now anyways.
                // Set to 1 initially, which is fine as long as the chunk size is not 1.
                let mut x_up = 1;
                let mut y_up = 1;
                let mut z_up = 1;
                for z in 0..=1 {
                    for y in 0..=1 {
                        for x in 0..=1 {
                            match Planet::planet_face_without_structure(
                                sx + x * CHUNK_DIMENSIONS,
                                sy + y * CHUNK_DIMENSIONS,
                                sz + z * CHUNK_DIMENSIONS,
                                s_width,
                                s_height,
                                s_length,
                            ) {
                                BlockFace::Top => y_up = CHUNK_DIMENSIONS - 1,
                                BlockFace::Bottom => y_up = 0,
                                BlockFace::Front => z_up = CHUNK_DIMENSIONS - 1,
                                BlockFace::Back => z_up = 0,
                                BlockFace::Right => x_up = CHUNK_DIMENSIONS - 1,
                                BlockFace::Left => x_up = 0,
                            }
                        }
                    }
                }

                let mut all_stone = true;
                let mut all_air = true;
                if z_up != 1 {
                    let z_down = if z_up != 0 { 0 } else { CHUNK_DIMENSIONS - 1 };
                    for y in 0..CHUNK_DIMENSIONS {
                        let actual_y: usize = sy + y;
                        for x in 0..CHUNK_DIMENSIONS {
                            let actual_x = sx + x;
                            if !chunk.has_block_at(x, y, z_up) {
                                generate_block(
                                    (x, y, z_up),
                                    (actual_x, actual_y, sz + z_up),
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
                            if chunk.block_at(x, y, z_up) != stone.id() {
                                all_stone = false;
                            }

                            if !chunk.has_block_at(x, y, z_down) {
                                generate_block(
                                    (x, y, z_down),
                                    (actual_x, actual_y, sz + z_down),
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
                            if chunk.block_at(x, y, z_down) != AIR_BLOCK_ID {
                                all_air = false;
                            }
                        }
                    }
                }

                if y_up != 1 {
                    let y_down = if y_up != 0 { 0 } else { CHUNK_DIMENSIONS - 1 };
                    for z in 0..CHUNK_DIMENSIONS {
                        let actual_z: usize = sz + z;
                        for x in 0..CHUNK_DIMENSIONS {
                            let actual_x = sx + x;
                            if !chunk.has_block_at(x, y_up, z) {
                                generate_block(
                                    (x, y_up, z),
                                    (actual_x, sy + y_up, actual_z),
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
                            if chunk.block_at(x, y_up, z) != stone.id() {
                                all_stone = false;
                            }

                            if !chunk.has_block_at(x, y_down, z) {
                                generate_block(
                                    (x, y_down, z),
                                    (actual_x, sy + y_down, actual_z),
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
                            if chunk.block_at(x, y_down, z) != AIR_BLOCK_ID {
                                all_air = false;
                            }
                        }
                    }
                }

                if x_up != 1 {
                    let x_down = if x_up != 0 { 0 } else { CHUNK_DIMENSIONS - 1 };
                    for z in 0..CHUNK_DIMENSIONS {
                        let actual_z: usize = sz + z;
                        for y in 0..CHUNK_DIMENSIONS {
                            let actual_y = sy + y;
                            if !chunk.has_block_at(x_up, y, z) {
                                generate_block(
                                    (x_up, y, z),
                                    (sx + x_up, actual_y, actual_z),
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
                            if chunk.block_at(x_up, y, z) != stone.id() {
                                all_stone = false;
                            }

                            if !chunk.has_block_at(x_down, y, z) {
                                generate_block(
                                    (x_down, y, z),
                                    (sx + x_down, actual_y, actual_z),
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
                            if chunk.block_at(x_down, y, z) != AIR_BLOCK_ID {
                                all_air = false;
                            }
                        }
                    }
                }

                if all_air {
                    (chunk, structure_entity)
                } else if all_stone {
                    for z in 0..CHUNK_DIMENSIONS {
                        for y in 0..CHUNK_DIMENSIONS {
                            for x in 0..CHUNK_DIMENSIONS {
                                let block_up = Planet::planet_face_without_structure(
                                    x, y, z, s_width, s_height, s_length,
                                );
                                chunk.set_block_at(x, y, z, stone, block_up)
                            }
                        }
                    }
                    (chunk, structure_entity)
                } else {
                    // Interesting (non-uniform-block) chunk generation.
                    // I'm sorry for the repetitive code I'm about to write.
                    if y_up == CHUNK_DIMENSIONS - 1 && x_up == 1 && z_up == 1 {
                        // Top-only chunks. Chunks with more than 1 up are too hard, for now that's block by block.
                        for z in 0..CHUNK_DIMENSIONS {
                            let actual_z = sz + z;
                            for x in 0..CHUNK_DIMENSIONS {
                                y_up = do_top_face(
                                    (x, y_up, z),
                                    (sx + x, sy, actual_z),
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
                    } else if y_up == 0 && x_up == 1 && z_up == 1 {
                        // Top-only chunks. Chunks with more than 1 up are too hard, for now that's block by block.
                        y_up = 1;
                        for z in 0..CHUNK_DIMENSIONS {
                            let actual_z = sz + z;
                            for x in 0..CHUNK_DIMENSIONS {
                                y_up = do_bottom_face(
                                    (x, y_up, z),
                                    (sx + x, sy, actual_z),
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
                    } else if x_up == CHUNK_DIMENSIONS - 1 && y_up == 1 && z_up == 1 {
                        // Top-only chunks. Chunks with more than 1 up are too hard, for now that's block by block.
                        for z in 0..CHUNK_DIMENSIONS {
                            let actual_z = sz + z;
                            for y in 0..CHUNK_DIMENSIONS {
                                x_up = do_right_face(
                                    (x_up, y, z),
                                    (sx, sy + y, actual_z),
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
                    } else if x_up == 0 && y_up == 1 && z_up == 1 {
                        // Top-only chunks. Chunks with more than 1 up are too hard, for now that's block by block.
                        x_up = 1;
                        for z in 0..CHUNK_DIMENSIONS {
                            let actual_z = sz + z;
                            for y in 0..CHUNK_DIMENSIONS {
                                x_up = do_left_face(
                                    (x_up, y, z),
                                    (sx, sy + y, actual_z),
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
                    } else if z_up == CHUNK_DIMENSIONS - 1 && x_up == 1 && y_up == 1 {
                        // Top-only chunks. Chunks with more than 1 up are too hard, for now that's block by block.
                        for x in 0..CHUNK_DIMENSIONS {
                            let actual_x = sx + x;
                            for y in 0..CHUNK_DIMENSIONS {
                                z_up = do_front_face(
                                    (x, y, z_up),
                                    (actual_x, sy + y, sz),
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
                    } else if z_up == 0 && x_up == 1 && y_up == 1 {
                        // Top-only chunks. Chunks with more than 1 up are too hard, for now that's block by block.
                        z_up = 1;
                        for x in 0..CHUNK_DIMENSIONS {
                            let actual_x = sx + x;
                            for y in 0..CHUNK_DIMENSIONS {
                                z_up = do_back_face(
                                    (x, y, z_up),
                                    (actual_x, sy + y, sz),
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
                    // timer.log_duration("Grass Chunk: ");
                    (chunk, structure_entity)
                }
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
