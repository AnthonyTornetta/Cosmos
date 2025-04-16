use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    block::data::persistence::ChunkLoadBlockDataEvent,
    physics::location::Location,
    structure::{
        ChunkInitEvent, Structure, StructureTypeSet,
        events::StructureLoadedEvent,
        ship::{Ship, ship_builder::TShipBuilder},
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

use super::server_ship_builder::ServerShipBuilder;

fn on_blueprint_ship(mut query: Query<(&mut SerializedData, &Structure, &mut NeedsBlueprinted), With<Ship>>, mut commands: Commands) {
    for (mut s_data, structure, mut blueprint) in query.iter_mut() {
        blueprint.subdir_name = "ship".into();

        save_structure(structure, &mut s_data, &mut commands);
        s_data.serialize_data("cosmos:is_ship", &true);
    }
}

fn on_save_ship(mut query: Query<(&mut SerializedData, &Structure), (With<NeedsSaved>, With<Ship>)>, mut commands: Commands) {
    for (mut s_data, structure) in query.iter_mut() {
        save_structure(structure, &mut s_data, &mut commands);
        s_data.serialize_data("cosmos:is_ship", &true);
    }
}

fn load_structure(
    entity: Entity,
    commands: &mut Commands,
    loc: Location,
    mut structure: Structure,
    s_data: &SerializedData,
    chunk_load_block_data_event_writer: &mut EventWriter<ChunkLoadBlockDataEvent>,
    chunk_set_event_writer: &mut EventWriter<ChunkInitEvent>,
    structure_loaded_event_writer: &mut EventWriter<StructureLoadedEvent>,
) {
    let mut entity_cmd = commands.entity(entity);

    let vel = s_data.deserialize_data("cosmos:velocity").unwrap_or(Velocity::zero());

    let builder = ServerShipBuilder::default();

    builder.insert_ship(&mut entity_cmd, loc, vel, &mut structure);

    let entity = entity_cmd.id();

    for res in structure.all_chunks_iter(false) {
        // This will always be true because include_empty is false
        if let ChunkIteratorResult::FilledChunk {
            position: coords,
            chunk: _,
        } = res
        {
            // Maybe wait till block data is set for this?
            chunk_set_event_writer.send(ChunkInitEvent {
                structure_entity: entity,
                coords,
                serialized_block_data: None,
            });
        }
    }

    entity_cmd.insert(structure);

    structure_loaded_event_writer.send(StructureLoadedEvent { structure_entity: entity });

    if let Ok(block_data) = s_data.deserialize_data::<AllBlockData>("cosmos:block_data") {
        for (chunk_coord, data) in block_data.0 {
            chunk_load_block_data_event_writer.send(ChunkLoadBlockDataEvent {
                data,
                chunk: chunk_coord,
                structure_entity: entity,
            });
        }
    }
}

fn on_load_ship_blueprint(
    query: Query<(Entity, &SerializedData, &NeedsBlueprintLoaded)>,
    mut commands: Commands,
    mut chunk_load_block_data_event_writer: EventWriter<ChunkLoadBlockDataEvent>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
    mut structure_loaded_event_writer: EventWriter<StructureLoadedEvent>,
) {
    for (entity, s_data, needs_blueprinted) in query.iter() {
        if s_data.deserialize_data::<bool>("cosmos:is_ship").unwrap_or(false) {
            if let Ok(structure) = s_data.deserialize_data::<Structure>("cosmos:structure") {
                info!("Loading ship blueprint!");
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
}

fn on_load_ship(
    query: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    mut commands: Commands,
    mut chunk_load_block_data_event_writer: EventWriter<ChunkLoadBlockDataEvent>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
    mut structure_loaded_event_writer: EventWriter<StructureLoadedEvent>,
) {
    for (entity, s_data) in query.iter() {
        if s_data.deserialize_data::<bool>("cosmos:is_ship").unwrap_or(false) {
            if let Ok(structure) = s_data.deserialize_data::<Structure>("cosmos:structure") {
                let loc = s_data
                    .deserialize_data("cosmos:location")
                    .expect("Every ship should have a location when saved!");

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
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        SAVING_SCHEDULE,
        (
            on_blueprint_ship.in_set(BlueprintingSystemSet::DoBlueprinting),
            on_save_ship.in_set(SavingSystemSet::DoSaving),
        )
            .in_set(StructureTypeSet::Ship),
    )
    .add_systems(
        LOADING_SCHEDULE,
        (
            on_load_ship_blueprint.in_set(LoadingBlueprintSystemSet::DoLoadingBlueprints),
            on_load_ship.in_set(LoadingSystemSet::DoLoading),
        )
            .in_set(StructureTypeSet::Ship),
    );
}
