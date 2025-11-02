//! Structure serialization/deserialization

use std::fs;

use bevy::prelude::*;
use bevy_rapier3d::plugin::RapierContextEntityLink;
use cosmos_core::{
    block::data::persistence::ChunkLoadBlockDataEvent,
    entities::EntityId,
    netty::{NoSendEntity, cosmos_encoder},
    physics::location::Location,
    structure::{
        ChunkInitEvent, Structure, StructureTypeSet,
        chunk::{Chunk, ChunkEntity, netty::SerializedChunkBlockData},
        coordinates::{ChunkCoordinate, CoordinateType},
        dynamic_structure::DynamicStructure,
        loading::StructureLoadingSet,
        planet::Planet,
    },
};
use serde::{Deserialize, Serialize};

use crate::persistence::{
    SaveFileIdentifier, SerializedData,
    loading::{LoadingSystemSet, NeedsLoaded},
    saving::{NeedsSaved, SAVING_SCHEDULE, SavingSystemSet},
};

use super::{biosphere::biosphere_generation::BiosphereGenerationSet, generation::planet_generator::ChunkNeedsGenerated};

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
                dimensions: dynamic_planet.chunk_dimensions(),
                temperature: planet.temperature(),
            },
        );
    }
}

fn generate_planet(entity: Entity, s_data: &SerializedData, planet_save_data: PlanetSaveData, commands: &mut Commands) {
    let structure = Structure::Dynamic(DynamicStructure::new(planet_save_data.dimensions));

    let mut entity_cmd = commands.entity(entity);

    let location: Location = s_data
        .deserialize_data("cosmos:location")
        .expect("Every planet should have a location when saved!");

    entity_cmd.insert((structure, Planet::new(planet_save_data.temperature), location));
}

fn on_load_planet_structure(query: Query<(Entity, &SerializedData), With<NeedsLoaded>>, mut commands: Commands) {
    for (entity, s_data) in query.iter() {
        if let Ok(planet_save_data) = s_data.deserialize_data::<PlanetSaveData>("cosmos:planet") {
            generate_planet(entity, s_data, planet_save_data, &mut commands);
        }
    }
}

#[derive(Debug, Component)]
/// This is responsible for signifying that a chunk needs either generated or loaded from disk
pub(super) struct ChunkNeedsPopulated {
    pub chunk_coords: ChunkCoordinate,
    pub structure_entity: Entity,
}

fn add_entity_id_to_new_structures(created: Query<Entity, (Added<Structure>, Without<EntityId>)>, mut commands: Commands) {
    for ent in created.iter() {
        commands.entity(ent).insert(EntityId::generate());
    }
}

fn populate_chunks(
    q_chunk_needs_populated: Query<(Entity, &ChunkNeedsPopulated)>,
    q_structure: Query<(&EntityId, Option<&SaveFileIdentifier>, &Location, &RapierContextEntityLink)>,
    mut commands: Commands,
) {
    for (entity, needs) in q_chunk_needs_populated.iter() {
        let Ok((entity_id, structure_svi, loc, physics_world)) = q_structure.get(needs.structure_entity) else {
            commands.entity(entity).remove::<ChunkNeedsPopulated>();

            continue;
        };

        let (cx, cy, cz): (CoordinateType, CoordinateType, CoordinateType) = needs.chunk_coords.into();

        let svi = if let Some(structure_svi) = structure_svi {
            SaveFileIdentifier::as_child(format!("{cx}_{cy}_{cz}"), structure_svi.clone())
        } else {
            SaveFileIdentifier::as_child(
                format!("{cx}_{cy}_{cz}"),
                SaveFileIdentifier::new(Some(loc.sector()), *entity_id, None),
            )
        };

        if let Ok(chunk) = fs::read(svi.get_save_file_path()) {
            if chunk.is_empty() {
                // This can happen if the file is currently being saved, just try again next frame or whenever it's available
                continue;
            }

            let serialized_data = cosmos_encoder::deserialize::<SerializedData>(&chunk).unwrap_or_else(|_| {
                panic!(
                    "Error parsing chunk @ {cx} {cy} {cz} ({svi:?}) - is the file corrupted? File len: {}",
                    chunk.len()
                )
            });

            commands
                .entity(entity)
                .insert((
                    serialized_data,
                    NeedsLoaded,
                    Name::new("Needs Loaded Chunk"),
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
                .insert((
                    ChunkNeedsGenerated {
                        coords: needs.chunk_coords,
                        structure_entity: needs.structure_entity,
                    },
                    Name::new("Needs Generated Chunk"),
                ))
                .remove::<ChunkNeedsPopulated>();
        }
    }
}

fn load_chunk(
    q_chunk_needs_loaded: Query<(Entity, &SerializedData, &ChunkEntity), With<NeedsLoaded>>,
    mut q_structure: Query<&mut Structure>,
    mut evw_chunk_init: MessageWriter<ChunkInitEvent>,
    mut evw_chunk_load_block_data: MessageWriter<ChunkLoadBlockDataEvent>,
    mut commands: Commands,
) {
    for (entity, sd, ce) in q_chunk_needs_loaded.iter() {
        let Ok(chunk) = sd.deserialize_data::<Chunk>("cosmos:chunk") else {
            continue;
        };

        let Ok(mut structure) = q_structure.get_mut(ce.structure_entity) else {
            continue;
        };

        let coords = chunk.chunk_coordinates();

        commands
            .entity(entity)
            .insert(Transform::from_translation(structure.chunk_relative_position(coords)));

        structure.set_chunk_entity(coords, entity);

        structure.set_chunk(chunk);

        evw_chunk_init.write(ChunkInitEvent {
            structure_entity: ce.structure_entity,
            coords,
            serialized_block_data: None,
        });

        // Block data is stored per-chunk as `SerializedChunkBlockData` on dynamic structures,
        // instead of fixed structures storing it as `AllBlockData` on the structure itself.
        if let Ok(data) = sd.deserialize_data::<SerializedChunkBlockData>("cosmos:block_data") {
            evw_chunk_load_block_data.write(ChunkLoadBlockDataEvent {
                data,
                chunk: coords,
                structure_entity: ce.structure_entity,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(SAVING_SCHEDULE, on_save_structure.in_set(SavingSystemSet::DoSaving))
        .add_systems(
            FixedUpdate,
            (
                add_entity_id_to_new_structures.in_set(StructureLoadingSet::CreateChunkEntities),
                populate_chunks.in_set(StructureLoadingSet::CreateChunkEntities).before(load_chunk),
                load_chunk
                    .in_set(StructureLoadingSet::LoadChunkBlocks)
                    .ambiguous_with(BiosphereGenerationSet::GenerateChunkFeatures),
                on_load_planet_structure.in_set(LoadingSystemSet::DoLoading),
            )
                .in_set(StructureTypeSet::Planet),
        );
}
