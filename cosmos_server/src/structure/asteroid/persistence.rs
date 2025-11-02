use bevy::prelude::*;
use cosmos_core::{
    block::data::persistence::ChunkLoadBlockDataEvent,
    physics::location::Location,
    prelude::StructureLoadingSet,
    structure::{
        ChunkInitEvent, Structure, StructureTypeSet,
        asteroid::{Asteroid, MovingAsteroid},
        blueprint::BlueprintType,
        events::StructureLoadedEvent,
        structure_iterator::ChunkIteratorResult,
    },
};

use crate::{
    persistence::{
        SerializedData,
        loading::{LOADING_SCHEDULE, LoadingBlueprintSystemSet, LoadingSystemSet, NeedsBlueprintLoaded, NeedsLoaded},
        make_persistent::{DefaultPersistentComponent, make_persistent},
        saving::{BlueprintingSystemSet, NeedsBlueprinted, NeedsSaved, SAVING_SCHEDULE, SavingSystemSet},
    },
    structure::persistence::{chunk::AllBlockData, save_structure},
};

fn on_blueprint_asteroid(mut query: Query<(&mut SerializedData, &Structure, &mut NeedsBlueprinted, &Asteroid)>, mut commands: Commands) {
    for (mut s_data, structure, mut blueprint, asteroid) in query.iter_mut() {
        blueprint.blueprint_type = Some(BlueprintType::Asteroid);

        save_structure(structure, &mut s_data, &mut commands);
        s_data.serialize_data("cosmos:asteroid", &asteroid.temperature());
    }
}

fn on_save_asteroid(mut query: Query<(&mut SerializedData, &Structure, &Asteroid), With<NeedsSaved>>, mut commands: Commands) {
    for (mut s_data, structure, asteroid) in query.iter_mut() {
        save_structure(structure, &mut s_data, &mut commands);
        s_data.serialize_data("cosmos:asteroid", &asteroid.temperature());
    }
}

fn load_structure(
    entity: Entity,
    commands: &mut Commands,
    loc: Location,
    structure: Structure,
    s_data: &SerializedData,
    temperature: f32,
    chunk_load_block_data_event_writer: &mut MessageWriter<ChunkLoadBlockDataEvent>,
    chunk_set_event_writer: &mut MessageWriter<ChunkInitEvent>,
    structure_loaded_event_writer: &mut MessageWriter<StructureLoadedEvent>,
) {
    let mut entity_cmd = commands.entity(entity);

    let entity = entity_cmd.id();

    for res in structure.all_chunks_iter(false) {
        // This will always be true because include_empty is false
        if let ChunkIteratorResult::FilledChunk {
            position: coords,
            chunk: _,
        } = res
        {
            // Maybe wait till block data is set for this?
            chunk_set_event_writer.write(ChunkInitEvent {
                structure_entity: entity,
                coords,
                serialized_block_data: None,
            });
        }
    }

    entity_cmd.insert((structure, Asteroid::new(temperature), loc));

    structure_loaded_event_writer.write(StructureLoadedEvent { structure_entity: entity });

    if let Ok(block_data) = s_data.deserialize_data::<AllBlockData>("cosmos:block_data") {
        for (chunk_coord, data) in block_data.0 {
            chunk_load_block_data_event_writer.write(ChunkLoadBlockDataEvent {
                data,
                chunk: chunk_coord,
                structure_entity: entity,
            });
        }
    }
}

fn on_load_asteroid_blueprint(
    query: Query<(Entity, &SerializedData, &NeedsBlueprintLoaded)>,
    mut commands: Commands,
    mut chunk_load_block_data_event_writer: MessageWriter<ChunkLoadBlockDataEvent>,
    mut chunk_set_event_writer: MessageWriter<ChunkInitEvent>,
    mut structure_loaded_event_writer: MessageWriter<StructureLoadedEvent>,
) {
    for (entity, s_data, needs_blueprinted) in query.iter() {
        if let Ok(temperature) = s_data.deserialize_data::<f32>("cosmos:asteroid")
            && let Ok(structure) = s_data.deserialize_data::<Structure>("cosmos:structure")
        {
            load_structure(
                entity,
                &mut commands,
                needs_blueprinted.spawn_at,
                structure,
                s_data,
                temperature,
                &mut chunk_load_block_data_event_writer,
                &mut chunk_set_event_writer,
                &mut structure_loaded_event_writer,
            );
        }
    }
}

fn on_load_asteroid(
    query: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    mut commands: Commands,
    mut chunk_load_block_data_event_writer: MessageWriter<ChunkLoadBlockDataEvent>,
    mut chunk_set_event_writer: MessageWriter<ChunkInitEvent>,
    mut structure_loaded_event_writer: MessageWriter<StructureLoadedEvent>,
) {
    for (entity, s_data) in query.iter() {
        if let Ok(temperature) = s_data.deserialize_data::<f32>("cosmos:asteroid")
            && let Ok(structure) = s_data.deserialize_data::<Structure>("cosmos:structure")
        {
            let loc = s_data
                .deserialize_data("cosmos:location")
                .expect("Every asteroid should have a location when saved!");

            load_structure(
                entity,
                &mut commands,
                loc,
                structure,
                s_data,
                temperature,
                &mut chunk_load_block_data_event_writer,
                &mut chunk_set_event_writer,
                &mut structure_loaded_event_writer,
            );
        }
    }
}

impl DefaultPersistentComponent for MovingAsteroid {}

pub(super) fn register(app: &mut App) {
    make_persistent::<MovingAsteroid>(app);

    app.add_systems(
        SAVING_SCHEDULE,
        (
            on_blueprint_asteroid
                .in_set(BlueprintingSystemSet::DoBlueprinting)
                .in_set(StructureTypeSet::Asteroid)
                .ambiguous_with(BlueprintingSystemSet::DoBlueprinting),
            on_save_asteroid
                .in_set(SavingSystemSet::DoSaving)
                .in_set(StructureTypeSet::Asteroid)
                .ambiguous_with(SavingSystemSet::DoSaving),
        ),
    )
    .add_systems(
        LOADING_SCHEDULE,
        (
            on_load_asteroid_blueprint
                .in_set(LoadingBlueprintSystemSet::DoLoadingBlueprints)
                .in_set(StructureLoadingSet::LoadStructure)
                .in_set(StructureTypeSet::Asteroid),
            on_load_asteroid
                .in_set(LoadingSystemSet::DoLoading)
                .in_set(StructureLoadingSet::LoadStructure)
                .in_set(StructureTypeSet::Asteroid),
        ),
    );
}
