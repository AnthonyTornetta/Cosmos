use std::ops::RangeBounds;

use bevy::prelude::{App, Commands, Component, Entity, EventReader, EventWriter, Query, With};
use cosmos_core::{
    block::blocks::*,
    structure::{
        chunk::CHUNK_DIMENSIONS,
        planet::{planet_builder::PlanetBuilder, planet_builder_trait::TPlanetBuilder},
        structure::{ChunkSetEvent, Structure},
    },
};

use crate::structure::server_structure_builder::ServerStructureBuilder;

pub struct ServerPlanetBuilder {
    builder: PlanetBuilder<ServerStructureBuilder>,
}

impl Default for ServerPlanetBuilder {
    fn default() -> Self {
        Self {
            builder: PlanetBuilder::new(ServerStructureBuilder::default()),
        }
    }
}

pub struct GeneratePlanetChunkEvent {
    x: usize,
    y: usize,
    z: usize,
    structure_entity: Entity,
}

#[derive(Component)]
struct NeedsGenerated;

impl TPlanetBuilder for ServerPlanetBuilder {
    fn create(
        &self,
        entity: &mut bevy::ecs::system::EntityCommands,
        transform: bevy::prelude::Transform,
        structure: &mut cosmos_core::structure::structure::Structure,
    ) {
        self.builder.create(entity, transform, structure);

        entity.insert(NeedsGenerated);
    }
}

fn check_needs_generated(
    mut commands: Commands,
    query: Query<&Structure, (With<NeedsGenerated>, With<Structure>)>,
    mut event_writer: EventWriter<GeneratePlanetChunkEvent>,
) {
    for s in query.iter() {
        for z in 0..s.length() {
            for y in 0..s.height() {
                for x in 0..s.width() {
                    event_writer.send(GeneratePlanetChunkEvent {
                        x,
                        y,
                        z,
                        structure_entity: s.get_entity().unwrap(),
                    });
                }
            }
        }

        commands
            .entity(s.get_entity().unwrap())
            .remove::<NeedsGenerated>();
    }
}

pub fn register(app: &mut App) {
    app.add_event::<GeneratePlanetChunkEvent>();
    app.add_system(generate_planet);
    app.add_system(check_needs_generated);
}

pub fn generate_planet(
    mut query: Query<&mut Structure>,
    mut events: EventReader<GeneratePlanetChunkEvent>,
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
