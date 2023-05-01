//! Creates a grass planet

use bevy::prelude::{
    App, Component, Entity, EventReader, EventWriter, IntoSystemConfigs, OnUpdate, Query, Res,
};
use cosmos_core::{
    block::Block,
    registry::Registry,
    structure::{
        chunk::{Chunk, CHUNK_DIMENSIONS},
        ChunkInitEvent, Structure,
    },
    utils::{resource_wrapper::ResourceWrapper, timer::UtilsTimer},
};
use noise::NoiseFn;
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};

use crate::structure::planet::generation::planet_generator::check_needs_generated_system;
use crate::GameState;

use super::{TBiosphere, TGenerateChunkEvent};

#[derive(Component, Debug)]
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

const AMPLITUDE: f64 = 13.0;
const DELTA: f64 = 0.05;

fn generate_planet(
    mut query: Query<&mut Structure>,
    mut events: EventReader<GrassChunkNeedsGeneratedEvent>,
    mut event_writer: EventWriter<ChunkInitEvent>,
    noise_generastor: Res<ResourceWrapper<noise::OpenSimplex>>,
    blocks: Res<Registry<Block>>,
) {
    let timer = UtilsTimer::start();

    let mut chunks = events
        .iter()
        .map(|ev| {
            if let Ok(mut structure) = query.get_mut(ev.structure_entity) {
                if let Some(chunk) = structure.take_chunk(ev.x, ev.y, ev.z) {
                    Some((ev.structure_entity, chunk))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<(Entity, Chunk)>>();

    chunks.par_iter_mut().for_each(|(structure_entity, chunk)| {
        let Ok(structure) = query.get(*structure_entity) else {
            return;
        };

        let grass = blocks.from_id("cosmos:grass").unwrap();
        let dirt = blocks.from_id("cosmos:dirt").unwrap();
        let stone = blocks.from_id("cosmos:stone").unwrap();

        let s_height = structure.blocks_height();

        let middle_air_start = s_height - 23;

        for z in 0..CHUNK_DIMENSIONS {
            let actual_z = chunk.structure_z() * CHUNK_DIMENSIONS + z;
            for x in 0..CHUNK_DIMENSIONS {
                let actual_x = chunk.structure_x() * CHUNK_DIMENSIONS + x;

                let max_y = (middle_air_start as f64
                    + noise_generastor
                        .0
                        .get([actual_x as f64 * DELTA, actual_z as f64 * DELTA])
                        * AMPLITUDE)
                    .round() as usize;

                let stone_range = 0..(max_y - 5);
                let dirt_range = (max_y - 5)..(max_y - 1);
                let grass_range = (max_y - 1)..max_y;

                let actual_y = chunk.structure_y() * CHUNK_DIMENSIONS;

                for y in 0..CHUNK_DIMENSIONS.min(max_y) {
                    let actual_y = actual_y + y;

                    if !chunk.has_block_at(x, y, z) {
                        if grass_range.contains(&actual_y) {
                            chunk.set_block_at(x, y, z, grass);
                        } else if dirt_range.contains(&actual_y) {
                            chunk.set_block_at(x, y, z, dirt);
                        } else if stone_range.contains(&actual_y) {
                            chunk.set_block_at(x, y, z, stone);
                        }
                    }
                }
            }
        }
    });

    let len = chunks.len();

    for (structure_entity, chunk) in chunks {
        if let Ok(mut structure) = query.get_mut(structure_entity) {
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
    app.add_event::<GrassChunkNeedsGeneratedEvent>();
    app.add_systems(
        (
            generate_planet,
            check_needs_generated_system::<GrassChunkNeedsGeneratedEvent, GrassBiosphereMarker>,
        )
            .in_set(OnUpdate(GameState::Playing)),
    );
}
