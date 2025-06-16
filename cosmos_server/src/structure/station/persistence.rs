use bevy::prelude::*;
use cosmos_core::{
    block::data::persistence::ChunkLoadBlockDataEvent,
    physics::location::Location,
    structure::{
        ChunkInitEvent, Structure, StructureTypeSet, events::StructureLoadedEvent, station::Station,
        structure_iterator::ChunkIteratorResult,
    },
};

use crate::{
    persistence::{
        SerializedData,
        loading::{LOADING_SCHEDULE, LoadingBlueprintSystemSet, LoadingSystemSet, NeedsBlueprintLoaded, NeedsLoaded},
        saving::{BlueprintingSystemSet, NeedsBlueprinted, NeedsSaved, SAVING_SCHEDULE, SavingSystemSet},
    },
    structure::persistence::{chunk::AllBlockData, save_structure},
};

fn on_blueprint_structure(
    mut query: Query<(&mut SerializedData, &Structure, &mut NeedsBlueprinted), With<Station>>,
    mut commands: Commands,
) {
    for (mut s_data, structure, mut blueprint) in query.iter_mut() {
        blueprint.subdir_name = "station".into();

        save_structure(structure, &mut s_data, &mut commands);
        s_data.serialize_data("cosmos:is_station", &true);
    }
}

fn on_save_structure(mut query: Query<(&mut SerializedData, &Structure), (With<NeedsSaved>, With<Station>)>, mut commands: Commands) {
    for (mut s_data, structure) in query.iter_mut() {
        save_structure(structure, &mut s_data, &mut commands);
        s_data.serialize_data("cosmos:is_station", &true);
    }
}

fn load_structure(
    entity: Entity,
    commands: &mut Commands,
    loc: Location,
    structure: Structure,
    s_data: &SerializedData,
    chunk_load_block_data_event_writer: &mut EventWriter<ChunkLoadBlockDataEvent>,
    chunk_set_event_writer: &mut EventWriter<ChunkInitEvent>,
    structure_loaded_event_writer: &mut EventWriter<StructureLoadedEvent>,
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
        } else {
            unreachable!("This will never execute because the `all_chunks_iter` include_empty is false");
        }
    }

    entity_cmd.insert((structure, Station, loc));

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

fn on_load_blueprint(
    query: Query<(Entity, &SerializedData, &NeedsBlueprintLoaded)>,
    mut commands: Commands,
    mut chunk_load_block_data_event_writer: EventWriter<ChunkLoadBlockDataEvent>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
    mut structure_loaded_event_writer: EventWriter<StructureLoadedEvent>,
) {
    for (entity, s_data, needs_blueprinted) in query.iter() {
        if s_data.deserialize_data::<bool>("cosmos:is_station").unwrap_or(false)
            && let Ok(structure) = s_data.deserialize_data::<Structure>("cosmos:structure") {
                load_structure(
                    entity,
                    &mut commands,
                    needs_blueprinted.spawn_at,
                    structure,
                    s_data,
                    &mut chunk_load_block_data_event_writer,
                    &mut chunk_set_event_writer,
                    &mut structure_loaded_event_writer,
                );
            }
    }
}

fn on_load_structure(
    query: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    mut commands: Commands,
    mut chunk_load_block_data_event_writer: EventWriter<ChunkLoadBlockDataEvent>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
    mut structure_loaded_event_writer: EventWriter<StructureLoadedEvent>,
) {
    for (entity, s_data) in query.iter() {
        if s_data.deserialize_data::<bool>("cosmos:is_station").unwrap_or(false)
            && let Ok(structure) = s_data.deserialize_data::<Structure>("cosmos:structure") {
                let loc = s_data
                    .deserialize_data("cosmos:location")
                    .expect("Every station should have a location when saved!");

                load_structure(
                    entity,
                    &mut commands,
                    loc,
                    structure,
                    s_data,
                    &mut chunk_load_block_data_event_writer,
                    &mut chunk_set_event_writer,
                    &mut structure_loaded_event_writer,
                );
            }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        SAVING_SCHEDULE,
        (
            on_blueprint_structure.in_set(BlueprintingSystemSet::DoBlueprinting),
            on_save_structure.in_set(SavingSystemSet::DoSaving),
        )
            .in_set(StructureTypeSet::Station),
    )
    .add_systems(
        LOADING_SCHEDULE,
        (
            on_load_blueprint.in_set(LoadingBlueprintSystemSet::DoLoadingBlueprints),
            on_load_structure.in_set(LoadingSystemSet::DoLoading),
        )
            .in_set(StructureTypeSet::Station),
    );
}
