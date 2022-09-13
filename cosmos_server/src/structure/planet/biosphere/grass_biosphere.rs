use bevy::prelude::{App, Component, Entity, EventReader, EventWriter, Query};
use cosmos_core::{
    block::blocks::*,
    structure::{
        chunk::CHUNK_DIMENSIONS,
        structure::{ChunkSetEvent, Structure},
    },
};

use super::biosphere::{TBiosphere, TGenerateChunkEvent};

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

pub fn generate_planet(
    mut query: Query<&mut Structure>,
    mut events: EventReader<GrassChunkNeedsGeneratedEvent>,
    mut event_writer: EventWriter<ChunkSetEvent>,
) {
    for ev in events.iter() {
        let mut structure = query.get_mut(ev.structure_entity.clone()).unwrap();

        let (start_x, start_y, start_z) = (
            ev.x * CHUNK_DIMENSIONS,
            ev.y * CHUNK_DIMENSIONS,
            ev.z * CHUNK_DIMENSIONS,
        );

        println!("Generated structure's chunk!");

        let s_height = structure.height() * CHUNK_DIMENSIONS;

        let middle_air_start = s_height - 20;

        for z in start_z..(start_z + CHUNK_DIMENSIONS) {
            for x in start_x..(start_x + CHUNK_DIMENSIONS) {
                let y_here = (s_height + (7.0 * ((x + z) as f32 * 0.1).sin()).round() as usize)
                    - middle_air_start;

                let stone_range = 0..(y_here - 5);
                let dirt_range = (y_here - 5)..(y_here - 1);
                let grass_range = (y_here - 1)..y_here;

                for y in start_y..((start_y + CHUNK_DIMENSIONS).min(y_here)) {
                    if grass_range.contains(&y) {
                        structure.set_block_at(x, y, z, &GRASS, None);

                        // let mut rng = rand::thread_rng();

                        // let n1: u8 = rng.gen();

                        // if n1 < 1 {
                        //     for ty in (y + 1)..(y + 7) {
                        //         if ty != y + 6 {
                        //             structure.set_block_at(x, ty, z, &CHERRY_LOG, None);
                        //         } else {
                        //             structure.set_block_at(x, ty, z, &CHERRY_LEAF, None);
                        //         }

                        //         if ty > y + 2 {
                        //             let range;
                        //             if ty < y + 5 {
                        //                 range = -2..3;
                        //             } else {
                        //                 range = -1..2;
                        //             }

                        //             for tz in range.clone() {
                        //                 for tx in range.clone() {
                        //                     if tx == 0 && tz == 0
                        //                         || (tx + (x as i32) < 0
                        //                             || tz + (z as i32) < 0
                        //                             || ((tx + (x as i32)) as usize)
                        //                                 >= structure.width() * 32
                        //                             || ((tz + (z as i32)) as usize)
                        //                                 >= structure.length() * 32)
                        //                     {
                        //                         continue;
                        //                     }
                        //                     structure.set_block_at(
                        //                         (x as i32 + tx) as usize,
                        //                         ty,
                        //                         (z as i32 + tz) as usize,
                        //                         &CHERRY_LEAF,
                        //                         None,
                        //                     );
                        //                 }
                        //             }
                        //         }
                        //     }
                        // }
                    } else if dirt_range.contains(&y) {
                        structure.set_block_at(x, y, z, &DIRT, None);
                    } else if stone_range.contains(&y) {
                        structure.set_block_at(x, y, z, &STONE, None);
                    }
                }
            }
        }

        event_writer.send(ChunkSetEvent {
            structure_entity: ev.structure_entity.clone(),
            x: ev.x,
            y: ev.y,
            z: ev.z,
        })
    }
}

pub fn register(app: &mut App) {
    app.add_event::<GrassChunkNeedsGeneratedEvent>();
    app.add_system(generate_planet);
    app.add_system(
        crate::structure::planet::generation::planet_generator::check_needs_generated_system::<
            GrassChunkNeedsGeneratedEvent,
        >,
    );
}
