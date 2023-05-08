//! Creates a grass planet

use bevy::prelude::{
    App, Component, Entity, EventReader, EventWriter, IntoSystemConfig, OnUpdate, Query, Res,
};
use cosmos_core::{
    block::{Block, BlockFace},
    physics::location::{Location, SECTOR_DIMENSIONS},
    registry::Registry,
    structure::{
        chunk::{Chunk, CHUNK_DIMENSIONS},
        planet::Planet,
        ChunkInitEvent, Structure,
    },
    utils::{resource_wrapper::ResourceWrapper, timer::UtilsTimer},
};
use noise::NoiseFn;
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};

use crate::GameState;

use super::{register_biosphere, TBiosphere, TGenerateChunkEvent, TemperatureRange};

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
    x: usize,
    y: usize,
    z: usize,
    structure_x: f64,
    structure_y: f64,
    structure_z: f64,
    noise_generastor: &Res<ResourceWrapper<noise::OpenSimplex>>,
    middle_air_start: usize,
) -> usize {
    let mut depth: f64 = 0.0;
    for iteration in 1..=ITERATIONS {
        let iteration = iteration as f64;
        depth += noise_generastor.get([
            (x as f64 + structure_x) * (DELTA / iteration),
            (y as f64 + structure_y) * (DELTA / iteration),
            (z as f64 + structure_z) * (DELTA / iteration),
        ]) * AMPLITUDE
            * iteration;
    }
    return (middle_air_start as f64 + depth).round() as usize;
}

fn generate_planet(
    mut query: Query<(&mut Structure, &Location)>,
    mut events: EventReader<GrassChunkNeedsGeneratedEvent>,
    mut event_writer: EventWriter<ChunkInitEvent>,
    noise_generastor: Res<ResourceWrapper<noise::OpenSimplex>>,
    blocks: Res<Registry<Block>>,
) {
    let timer = UtilsTimer::start();

    let mut chunks = events
        .iter()
        .filter_map(|ev| {
            if let Ok((mut structure, _)) = query.get_mut(ev.structure_entity) {
                structure
                    .take_chunk(ev.x, ev.y, ev.z)
                    .map(|chunk| (ev.structure_entity, chunk))
            } else {
                None
            }
        })
        .collect::<Vec<(Entity, Chunk)>>();

    chunks.par_iter_mut().for_each(|(structure_entity, chunk)| {
        let Ok((structure, location)) = query.get(*structure_entity) else {
            return;
        };

        let grass = blocks.from_id("cosmos:grass").unwrap();
        let dirt = blocks.from_id("cosmos:dirt").unwrap();
        let stone = blocks.from_id("cosmos:stone").unwrap();

        let s_height = structure.blocks_height();

        let middle_air_start = s_height - CHUNK_DIMENSIONS * 5;

        let structure_z =
            (location.sector_z as f64) * SECTOR_DIMENSIONS as f64 + location.local.z as f64;
        let structure_y =
            (location.sector_y as f64) * SECTOR_DIMENSIONS as f64 + location.local.y as f64;
        let structure_x =
            (location.sector_x as f64) * SECTOR_DIMENSIONS as f64 + location.local.x as f64;

        for z in 0..CHUNK_DIMENSIONS {
            let actual_z = chunk.structure_z() * CHUNK_DIMENSIONS + z;
            for y in 0..CHUNK_DIMENSIONS {
                let actual_y: usize = chunk.structure_y() * CHUNK_DIMENSIONS + y;
                for x in 0..CHUNK_DIMENSIONS {
                    if chunk.has_block_at(x, y, z) {
                        continue;
                    }

                    let actual_x = chunk.structure_x() * CHUNK_DIMENSIONS + x;

                    let current_max = get_max_level(
                        actual_x,
                        actual_y,
                        actual_z,
                        structure_x,
                        structure_y,
                        structure_z,
                        &noise_generastor,
                        middle_air_start,
                    );

                    let current_height: usize;
                    let mut cover_x = actual_x;
                    let mut cover_y = actual_y;
                    let mut cover_z = actual_z;
                    let block_up = Planet::planet_face(structure, actual_x, actual_y, actual_z);
                    match block_up {
                        BlockFace::Top => {
                            cover_y += 1;
                            current_height = actual_y;
                        }
                        BlockFace::Bottom => {
                            cover_y -= 1;
                            current_height = structure.blocks_height() - actual_y;
                        }
                        BlockFace::Front => {
                            cover_z += 1;
                            current_height = actual_z;
                        }
                        BlockFace::Back => {
                            cover_z -= 1;
                            current_height = structure.blocks_length() - actual_z;
                        }
                        BlockFace::Right => {
                            cover_x += 1;
                            current_height = actual_x;
                        }
                        BlockFace::Left => {
                            cover_x -= 1;
                            current_height = structure.blocks_length() - actual_x;
                        }
                    }

                    if current_height < current_max - STONE_LIMIT {
                        chunk.set_block_at(x, y, z, stone, block_up);
                    } else if current_height < current_max {
                        // Getting the noise values for the "covering" block.
                        let cover_height = current_height + 1;
                        let cover_max = get_max_level(
                            cover_x,
                            cover_y,
                            cover_z,
                            structure_x,
                            structure_y,
                            structure_z,
                            &noise_generastor,
                            middle_air_start,
                        );
                        if cover_height < cover_max {
                            // In dirt range and covered -> dirt.
                            chunk.set_block_at(x, y, z, dirt, block_up)
                        } else {
                            // In dirt range and uncovered -> grass.
                            chunk.set_block_at(x, y, z, grass, block_up)
                        }
                    }
                }
            }
        }
    });

    let len = chunks.len();

    for (structure_entity, chunk) in chunks {
        if let Ok((mut structure, _)) = query.get_mut(structure_entity) {
            event_writer.send(ChunkInitEvent {
                structure_entity,
                x: chunk.structure_x(),
                y: chunk.structure_y(),
                z: chunk.structure_z(),
            });

            structure.set_chunk(chunk);
        }
    }

    if len != 0 {
        timer.log_duration(&format!("Generated {len} grass chunks in"));
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent>(
        app,
        "cosmos:biosphere_grass",
        TemperatureRange::new(0.0, 1000000000.0),
    );

    app.add_system(generate_planet.in_set(OnUpdate(GameState::Playing)));
}
