//! Used just for testing, this makes a planet all stone

use bevy::prelude::{
    App, Component, Entity, EventReader, EventWriter, IntoSystemConfig, OnUpdate, Query, Res,
};
use cosmos_core::{
    block::Block,
    registry::Registry,
    structure::{
        chunk::{Chunk, CHUNK_DIMENSIONS},
        ChunkInitEvent, Structure,
    },
    utils::timer::UtilsTimer,
};
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};

use crate::GameState;

use super::{register_biosphere, TBiosphere, TGenerateChunkEvent};

#[derive(Component, Debug, Default)]
/// Used just for testing, this makes a planet all stone
pub struct TestStoneBiosphereMarker;

/// Used just for testing, this makes a planet all stone
pub struct TestStoneChunkNeedsGeneratedEvent {
    x: usize,
    y: usize,
    z: usize,
    structure_entity: Entity,
}

impl TGenerateChunkEvent for TestStoneChunkNeedsGeneratedEvent {
    fn new(x: usize, y: usize, z: usize, structure_entity: Entity) -> Self {
        Self {
            x,
            y,
            z,
            structure_entity,
        }
    }
}

#[derive(Default)]
/// Used just for testing, this makes a planet all stone
pub struct TestStoneBiosphere;

impl TBiosphere<TestStoneBiosphereMarker, TestStoneChunkNeedsGeneratedEvent>
    for TestStoneBiosphere
{
    fn get_marker_component(&self) -> TestStoneBiosphereMarker {
        TestStoneBiosphereMarker
    }

    fn get_generate_chunk_event(
        &self,
        x: usize,
        y: usize,
        z: usize,
        structure_entity: Entity,
    ) -> TestStoneChunkNeedsGeneratedEvent {
        TestStoneChunkNeedsGeneratedEvent::new(x, y, z, structure_entity)
    }
}

fn generate_planet(
    mut query: Query<&mut Structure>,
    mut events: EventReader<TestStoneChunkNeedsGeneratedEvent>,
    mut event_writer: EventWriter<ChunkInitEvent>,
    blocks: Res<Registry<Block>>,
) {
    let timer = UtilsTimer::start();

    let mut chunks = events
        .iter()
        .map(|ev: &TestStoneChunkNeedsGeneratedEvent| {
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

    chunks.par_iter_mut().for_each(|(_, chunk)| {
        let stone = blocks.from_id("cosmos:stone").unwrap();

        for z in 0..CHUNK_DIMENSIONS {
            for y in 0..CHUNK_DIMENSIONS {
                for x in 0..CHUNK_DIMENSIONS {
                    chunk.set_block_at(x, y, z, stone);
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
        timer.log_duration(&format!("Generated {len} chunks in"));
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<TestStoneBiosphereMarker, TestStoneChunkNeedsGeneratedEvent>(
        app,
        "cosmos:biosphere_test_stone",
    );

    app.add_system(generate_planet.in_set(OnUpdate(GameState::Playing)));
}
