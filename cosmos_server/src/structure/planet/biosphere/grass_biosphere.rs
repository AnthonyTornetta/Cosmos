use bevy::prelude::{App, Component, Entity, EventReader, EventWriter, Query, Res};
use cosmos_core::{
    block::blocks::Blocks,
    structure::{chunk::CHUNK_DIMENSIONS, events::ChunkSetEvent, structure::Structure},
    utils::resource_wrapper::ResourceWrapper,
};
use noise::NoiseFn;

use crate::structure::planet::generation::planet_generator::check_needs_generated_system;
use crate::{GameState, SystemSet};

use super::{TBiosphere, TGenerateChunkEvent};

#[derive(Component)]
pub struct GrassBiosphereMarker;

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

#[derive(Default)]
pub struct GrassBiosphere {}

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

const AMPLITUDE: f64 = 30.0;
const DELTA: f64 = 0.05;

pub fn generate_planet(
    mut query: Query<&mut Structure>,
    mut events: EventReader<GrassChunkNeedsGeneratedEvent>,
    mut event_writer: EventWriter<ChunkSetEvent>,
    noise_generastor: Res<ResourceWrapper<noise::OpenSimplex>>,
    blocks: Res<Blocks>,
) {
    for ev in events.iter() {
        println!("Generating grass chunk for {} {} {}!", ev.x, ev.y, ev.z);
        let mut structure = query.get_mut(ev.structure_entity).unwrap();

        let (start_x, start_y, start_z) = (
            ev.x * CHUNK_DIMENSIONS,
            ev.y * CHUNK_DIMENSIONS,
            ev.z * CHUNK_DIMENSIONS,
        );

        // let stone = blocks.block_from_id("cosmos:stone").unwrap();

        // for z in start_z..start_z + CHUNK_DIMENSIONS {
        //     for y in start_y..start_y + CHUNK_DIMENSIONS {
        //         for x in start_x..start_x + CHUNK_DIMENSIONS {
        //             // if x == y && y == z {
        //             structure.set_block_at(x, y, z, stone, &blocks, None);
        //             // }
        //         }
        //     }
        // }

        let grass = blocks.block_from_id("cosmos:grass").unwrap();
        let dirt = blocks.block_from_id("cosmos:dirt").unwrap();
        let stone = blocks.block_from_id("cosmos:stone").unwrap();

        let s_height = structure.blocks_height();

        let middle_air_start = s_height - 23;

        for z in start_z..(start_z + CHUNK_DIMENSIONS) {
            for x in start_x..(start_x + CHUNK_DIMENSIONS) {
                let y_here = (middle_air_start as f64
                    + noise_generastor.0.get([x as f64 * DELTA, z as f64 * DELTA]) * AMPLITUDE)
                    .round() as usize;

                let stone_range = 0..(y_here - 5);
                let dirt_range = (y_here - 5)..(y_here - 1);
                let grass_range = (y_here - 1)..y_here;

                for y in start_y..((start_y + CHUNK_DIMENSIONS).min(y_here)) {
                    if !structure.has_block_at(x, y, z) {
                        if grass_range.contains(&y) {
                            structure.set_block_at(x, y, z, grass, &blocks, None);
                        } else if dirt_range.contains(&y) {
                            structure.set_block_at(x, y, z, dirt, &blocks, None);
                        } else if stone_range.contains(&y) {
                            structure.set_block_at(x, y, z, stone, &blocks, None);
                        }
                    }
                }
            }
        }

        println!("Done generating {} {} {}!", ev.x, ev.y, ev.z);
        event_writer.send(ChunkSetEvent {
            structure_entity: ev.structure_entity,
            x: ev.x,
            y: ev.y,
            z: ev.z,
        });
    }
}

pub fn register(app: &mut App) {
    app.add_event::<GrassChunkNeedsGeneratedEvent>();
    app.add_system_set(
        SystemSet::on_update(GameState::Playing)
            .with_system(generate_planet)
            .with_system(check_needs_generated_system::<GrassChunkNeedsGeneratedEvent>),
    );
}
