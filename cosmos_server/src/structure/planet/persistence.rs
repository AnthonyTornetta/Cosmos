use std::fs;

use bevy::prelude::*;
use cosmos_core::{
    netty::cosmos_encoder,
    physics::{location::Location, player_world::PlayerWorld},
    structure::{
        chunk::{Chunk, ChunkEntity},
        planet::{planet_builder::TPlanetBuilder, Planet},
        ChunkInitEvent, Structure,
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    persistence::{
        loading::{begin_loading, done_loading, NeedsLoaded},
        saving::{begin_saving, done_saving, NeedsSaved},
        EntityId, SaveFileIdentifier, SerializedData,
    },
    structure::persistence::DelayedStructureLoadEvent,
};

use super::{
    generation::planet_generator::NeedsGenerated, server_planet_builder::ServerPlanetBuilder,
};

#[derive(Debug, Serialize, Deserialize)]
struct PlanetSaveData {
    width: usize,
    height: usize,
    length: usize,
}

fn on_save_structure(
    mut query: Query<(&mut SerializedData, &Structure), (With<NeedsSaved>, With<Planet>)>,
) {
    for (mut s_data, structure) in query.iter_mut() {
        s_data.serialize_data(
            "cosmos:planet",
            &PlanetSaveData {
                width: structure.chunks_width(),
                height: structure.chunks_height(),
                length: structure.chunks_length(),
            },
        );
        s_data.serialize_data("cosmos:is_planet", &true);
    }
}

fn on_load_structure(
    query: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    player_worlds: Query<&Location, With<PlayerWorld>>,
    mut event_writer: EventWriter<DelayedStructureLoadEvent>,
    mut commands: Commands,
) {
    for (entity, s_data) in query.iter() {
        if let Some(is_planet) = s_data.deserialize_data::<bool>("cosmos:is_planet") {
            if is_planet {
                if let Some(planet_save_data) =
                    s_data.deserialize_data::<PlanetSaveData>("cosmos:planet")
                {
                    let mut structure = Structure::new(
                        planet_save_data.width,
                        planet_save_data.height,
                        planet_save_data.length,
                    );

                    let mut best_loc = None;
                    let mut best_dist = f32::INFINITY;

                    let loc = s_data
                        .deserialize_data("cosmos:location")
                        .expect("Every planet should have a location when saved!");

                    for world_loc in player_worlds.iter() {
                        let dist = world_loc.distance_sqrd(&loc);
                        if dist < best_dist {
                            best_dist = dist;
                            best_loc = Some(world_loc);
                        }
                    }

                    if let Some(world_location) = best_loc {
                        let mut entity_cmd = commands.entity(entity);

                        let builder = ServerPlanetBuilder::default();

                        builder.insert_planet(&mut entity_cmd, loc, world_location, &mut structure);

                        let entity = entity_cmd.id();

                        event_writer.send(DelayedStructureLoadEvent(entity));

                        commands.entity(entity).insert(structure);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Component)]
pub struct NeedsPopulated {
    pub chunk_coords: (usize, usize, usize),
    pub structure_entity: Entity,
}

fn structure_created(
    created: Query<Entity, (Added<Structure>, Without<EntityId>)>,
    mut commands: Commands,
) {
    for ent in created.iter() {
        commands.entity(ent).insert(EntityId::generate());
    }
}

fn populate_chunks(
    query: Query<(Entity, &NeedsPopulated)>,
    structure_query: Query<(&EntityId, Option<&SaveFileIdentifier>, &Location)>,
    mut commands: Commands,
) {
    for (entity, needs) in query.iter() {
        commands.entity(entity).remove::<NeedsPopulated>();

        let Ok((entity_id, structure_svi, loc)) = structure_query.get(needs.structure_entity) else {
            continue;
        };

        let (cx, cy, cz) = needs.chunk_coords;

        let svi = if let Some(structure_svi) = structure_svi {
            SaveFileIdentifier::as_child(format!("{cx}_{cy}_{cz}"), structure_svi.clone())
        } else {
            SaveFileIdentifier::as_child(
                format!("{cx}_{cy}_{cz}"),
                SaveFileIdentifier::new(Some(loc.sector()), entity_id.clone()),
            )
        };

        if let Ok(chunk) = fs::read(svi.get_save_file_path()) {
            println!("Loading chunk @ {cx} {cy} {cz}");
            let serialized_data = cosmos_encoder::deserialize::<SerializedData>(&chunk)
                .expect("Error parsing chunk - is the file corrupted?");

            commands.entity(entity).insert((
                serialized_data,
                NeedsLoaded,
                ChunkEntity {
                    structure_entity: needs.structure_entity,
                    chunk_location: needs.chunk_coords,
                },
            ));
        } else {
            commands.entity(entity).insert(NeedsGenerated {
                chunk_coords: needs.chunk_coords,
                structure_entity: needs.structure_entity,
            });
        }
    }
}

fn load_chunk(
    query: Query<(Entity, &SerializedData, &ChunkEntity), With<NeedsLoaded>>,
    mut structure_query: Query<&mut Structure>,
    mut chunk_init_event: EventWriter<ChunkInitEvent>,
) {
    for (entity, sd, ce) in query.iter() {
        if let Some(chunk) = sd.deserialize_data::<Chunk>("cosmos:chunk") {
            if let Ok(mut structure) = structure_query.get_mut(ce.structure_entity) {
                let (cx, cy, cz) = (
                    chunk.structure_x(),
                    chunk.structure_y(),
                    chunk.structure_z(),
                );

                structure.set_chunk_entity(cx, cy, cz, entity);

                structure.set_chunk(chunk);

                println!("Loaded!! chunk @ {cx} {cy} {cz}");

                chunk_init_event.send(ChunkInitEvent {
                    structure_entity: ce.structure_entity,
                    x: cx,
                    y: cy,
                    z: cz,
                });
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems((structure_created, populate_chunks).chain())
        .add_system(on_save_structure.after(begin_saving).before(done_saving))
        .add_system(on_load_structure.after(begin_loading).before(done_loading))
        .add_system(load_chunk.after(begin_loading).before(done_loading));
}
