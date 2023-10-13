use std::fs;

use bevy::prelude::*;
use bevy_rapier3d::prelude::PhysicsWorld;
use cosmos_core::{
    netty::{cosmos_encoder, NoSendEntity},
    physics::location::Location,
    structure::{
        chunk::{Chunk, ChunkEntity},
        coordinates::{ChunkCoordinate, CoordinateType},
        dynamic_structure::DynamicStructure,
        planet::{planet_builder::TPlanetBuilder, Planet},
        ChunkInitEvent, Structure,
    },
};
use serde::{Deserialize, Serialize};

use crate::persistence::{
    loading::{begin_loading, done_loading, NeedsLoaded},
    saving::{begin_saving, done_saving, NeedsSaved},
    EntityId, SaveFileIdentifier, SerializedData,
};

use super::{generation::planet_generator::ChunkNeedsGenerated, server_planet_builder::ServerPlanetBuilder};

#[derive(Debug, Serialize, Deserialize)]
struct PlanetSaveData {
    dimensions: CoordinateType,
    temperature: f32,
}

fn on_save_structure(mut query: Query<(&mut SerializedData, &Structure, &Planet), With<NeedsSaved>>) {
    for (mut s_data, structure, planet) in query.iter_mut() {
        let Structure::Dynamic(dynamic_planet) = structure else {
            panic!("Planet must be dynamic!");
        };

        s_data.serialize_data(
            "cosmos:planet",
            &PlanetSaveData {
                dimensions: dynamic_planet.dimensions(),
                temperature: planet.temperature(),
            },
        );
        s_data.serialize_data("cosmos:is_planet", &true);
    }
}

fn generate_planet(entity: Entity, s_data: &SerializedData, planet_save_data: PlanetSaveData, commands: &mut Commands) {
    let mut structure = Structure::Dynamic(DynamicStructure::new(planet_save_data.dimensions));

    let mut entity_cmd = commands.entity(entity);

    let location: Location = s_data
        .deserialize_data("cosmos:location")
        .expect("Every planet should have a location when saved!");

    let builder = ServerPlanetBuilder::default();

    builder.insert_planet(&mut entity_cmd, location, &mut structure, Planet::new(planet_save_data.temperature));

    entity_cmd.insert(structure);
}

fn on_load_structure(query: Query<(Entity, &SerializedData), With<NeedsLoaded>>, mut commands: Commands) {
    for (entity, s_data) in query.iter() {
        if s_data.deserialize_data::<bool>("cosmos:is_planet").unwrap_or(false) {
            if let Some(planet_save_data) = s_data.deserialize_data::<PlanetSaveData>("cosmos:planet") {
                generate_planet(entity, s_data, planet_save_data, &mut commands);
            }
        }
    }
}

#[derive(Debug, Component)]
/// This is responsible for signifying that a chunk needs either generated or loaded from disk
pub(super) struct ChunkNeedsPopulated {
    pub chunk_coords: ChunkCoordinate,
    pub structure_entity: Entity,
}

fn structure_created(created: Query<Entity, (Added<Structure>, Without<EntityId>)>, mut commands: Commands) {
    for ent in created.iter() {
        commands.entity(ent).insert(EntityId::generate());
    }
}

fn populate_chunks(
    query: Query<(Entity, &ChunkNeedsPopulated)>,
    structure_query: Query<(&EntityId, Option<&SaveFileIdentifier>, &Location, &PhysicsWorld)>,
    mut commands: Commands,
) {
    for (entity, needs) in query.iter() {
        let Ok((entity_id, structure_svi, loc, physics_world)) = structure_query.get(needs.structure_entity) else {
            commands.entity(entity).remove::<ChunkNeedsPopulated>();

            continue;
        };

        let (cx, cy, cz): (CoordinateType, CoordinateType, CoordinateType) = needs.chunk_coords.into();

        let svi = if let Some(structure_svi) = structure_svi {
            SaveFileIdentifier::as_child(format!("{cx}_{cy}_{cz}"), structure_svi.clone())
        } else {
            SaveFileIdentifier::as_child(
                format!("{cx}_{cy}_{cz}"),
                SaveFileIdentifier::new(Some(loc.sector()), entity_id.clone(), None),
            )
        };

        if let Ok(chunk) = fs::read(svi.get_save_file_path()) {
            if chunk.is_empty() {
                // This can happen if the file is currently being saved, just try again next frame or whenever it's available
                continue;
            }

            let serialized_data = cosmos_encoder::deserialize::<SerializedData>(&chunk).unwrap_or_else(|_| {
                panic!(
                    "Error parsing chunk @ {cx} {cy} {cz} - is the file corrupted? File len: {}",
                    chunk.len()
                )
            });

            commands
                .entity(entity)
                .insert((
                    serialized_data,
                    NeedsLoaded,
                    NoSendEntity,
                    ChunkEntity {
                        structure_entity: needs.structure_entity,
                        chunk_location: needs.chunk_coords,
                    },
                    *physics_world,
                ))
                .remove::<ChunkNeedsPopulated>();
        } else {
            commands
                .entity(entity)
                .insert(ChunkNeedsGenerated {
                    coords: needs.chunk_coords,
                    structure_entity: needs.structure_entity,
                })
                .remove::<ChunkNeedsPopulated>();
        }
    }
}

fn load_chunk(
    query: Query<(Entity, &SerializedData, &ChunkEntity), With<NeedsLoaded>>,
    mut structure_query: Query<&mut Structure>,
    mut chunk_init_event: EventWriter<ChunkInitEvent>,
    mut commands: Commands,
) {
    for (entity, sd, ce) in query.iter() {
        if let Some(chunk) = sd.deserialize_data::<Chunk>("cosmos:chunk") {
            if let Ok(mut structure) = structure_query.get_mut(ce.structure_entity) {
                let coords = chunk.chunk_coordinates();

                commands.entity(entity).insert(PbrBundle {
                    transform: Transform::from_translation(structure.chunk_relative_position(coords)),
                    ..Default::default()
                });

                structure.set_chunk_entity(coords, entity);

                structure.set_chunk(chunk);

                chunk_init_event.send(ChunkInitEvent {
                    structure_entity: ce.structure_entity,
                    coords,
                });
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (structure_created, populate_chunks).chain())
        .add_systems(First, on_save_structure.after(begin_saving).before(done_saving))
        .add_systems(Update, on_load_structure.after(begin_loading).before(done_loading))
        .add_systems(Update, load_chunk.after(begin_loading).before(done_loading));
}
