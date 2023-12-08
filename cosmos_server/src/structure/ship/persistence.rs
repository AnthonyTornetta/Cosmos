use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    physics::location::Location,
    structure::{
        events::StructureLoadedEvent,
        loading::StructureLoadingSet,
        ship::{ship_builder::TShipBuilder, Ship},
        structure_iterator::ChunkIteratorResult,
        ChunkInitEvent, Structure,
    },
};

use crate::{
    persistence::{
        loading::{LoadingBlueprintSystemSet, LoadingSystemSet, NeedsBlueprintLoaded, NeedsLoaded},
        saving::{BlueprintingSystemSet, NeedsBlueprinted, NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
        SerializedData,
    },
    structure::persistence::{
        chunk::{AllBlockData, ChunkLoadBlockDataEvent},
        save_structure, BlockDataNeedsSavedThisIsStupidPleaseMakeThisAComponent, SuperDuperStupidGarbage,
    },
};

use super::server_ship_builder::ServerShipBuilder;

fn on_blueprint_structure(
    mut ev_writer: EventWriter<BlockDataNeedsSavedThisIsStupidPleaseMakeThisAComponent>,
    mut query: Query<(&mut SerializedData, &Structure, &mut NeedsBlueprinted), With<Ship>>,
    mut commands: Commands,
    mut garbage: ResMut<SuperDuperStupidGarbage>,
) {
    for (mut s_data, structure, mut blueprint) in query.iter_mut() {
        blueprint.subdir_name = "ship".into();

        save_structure(structure, &mut s_data, &mut commands, &mut ev_writer, &mut garbage);
        s_data.serialize_data("cosmos:is_ship", &true);
    }
}

fn on_save_structure(
    mut ev_writer: EventWriter<BlockDataNeedsSavedThisIsStupidPleaseMakeThisAComponent>,
    mut query: Query<(&mut SerializedData, &Structure), (With<NeedsSaved>, With<Ship>)>,
    mut commands: Commands,
    mut garbage: ResMut<SuperDuperStupidGarbage>,
) {
    for (mut s_data, structure) in query.iter_mut() {
        save_structure(structure, &mut s_data, &mut commands, &mut ev_writer, &mut garbage);
        s_data.serialize_data("cosmos:is_ship", &true);
    }
}

fn load_structure(
    entity: Entity,
    commands: &mut Commands,
    loc: Location,
    mut structure: Structure,
    s_data: &SerializedData,
    // event_writer: &mut EventWriter<DelayedStructureLoadEvent>,
    chunk_load_block_data_event_writer: &mut EventWriter<ChunkLoadBlockDataEvent>,
    chunk_set_event_writer: &mut EventWriter<ChunkInitEvent>,
    structure_loaded_event_writer: &mut EventWriter<StructureLoadedEvent>,
) {
    let mut entity_cmd = commands.entity(entity);

    let vel = s_data.deserialize_data("cosmos:velocity").unwrap_or(Velocity::zero());

    let builder = ServerShipBuilder::default();

    builder.insert_ship(&mut entity_cmd, loc, vel, &mut structure);

    let entity = entity_cmd.id();

    // event_writer.send(DelayedStructureLoadEvent(entity));

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
            });
        }
    }

    structure_loaded_event_writer.send(StructureLoadedEvent { structure_entity: entity });

    info!("LOGGING COMPONENTS");
    commands.entity(entity).insert(structure).log_components();

    if let Some(block_data) = s_data.deserialize_data::<AllBlockData>("cosmos:block_data") {
        for (chunk_coord, data) in block_data.0 {
            chunk_load_block_data_event_writer.send(ChunkLoadBlockDataEvent {
                data,
                chunk: chunk_coord,
                structure_entity: entity,
            });
        }
    }
}

fn on_load_blueprint(
    query: Query<(Entity, &SerializedData, &NeedsBlueprintLoaded), With<NeedsBlueprintLoaded>>,
    // mut event_writer: EventWriter<DelayedStructureLoadEvent>,
    mut commands: Commands,
    mut chunk_load_block_data_event_writer: EventWriter<ChunkLoadBlockDataEvent>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
    mut structure_loaded_event_writer: EventWriter<StructureLoadedEvent>,
) {
    for (entity, s_data, needs_blueprinted) in query.iter() {
        if s_data.deserialize_data::<bool>("cosmos:is_ship").unwrap_or(false) {
            if let Some(structure) = s_data.deserialize_data::<Structure>("cosmos:structure") {
                load_structure(
                    entity,
                    &mut commands,
                    needs_blueprinted.spawn_at,
                    structure,
                    s_data,
                    &mut chunk_load_block_data_event_writer,
                    // &mut event_writer,
                    &mut chunk_set_event_writer,
                    &mut structure_loaded_event_writer,
                );
            }
        }
    }
}

fn on_load_structure(
    query: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    // mut event_writer: EventWriter<DelayedStructureLoadEvent>,
    mut commands: Commands,
    mut chunk_load_block_data_event_writer: EventWriter<ChunkLoadBlockDataEvent>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
    mut structure_loaded_event_writer: EventWriter<StructureLoadedEvent>,
) {
    for (entity, s_data) in query.iter() {
        if s_data.deserialize_data::<bool>("cosmos:is_ship").unwrap_or(false) {
            if let Some(structure) = s_data.deserialize_data::<Structure>("cosmos:structure") {
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
                    // &mut event_writer,
                    &mut chunk_set_event_writer,
                    &mut structure_loaded_event_writer,
                );
            }
        }
    }
}

/// I hate this, but the only way to prevent issues with events is to delay the sending of the chunk init events
/// by 2 frames, so two events are needed to do this. This is really horrible, but the only way I can think of
/// to get this to work ;(
// #[derive(Debug, Event)]
// struct DelayedStructureLoadEvent(pub Entity);
// #[derive(Debug, Event)]
// struct EvenMoreDelayedStructureLoadEvent(Entity);

// fn delayed_structure_event(
//     mut event_reader: EventReader<DelayedStructureLoadEvent>,
//     mut event_writer: EventWriter<EvenMoreDelayedStructureLoadEvent>,
// ) {
//     for ev in event_reader.read() {
//         event_writer.send(EvenMoreDelayedStructureLoadEvent(ev.0));
//     }
// }

// fn even_more_delayed_structure_event(
//     mut event_reader: EventReader<EvenMoreDelayedStructureLoadEvent>,
//     mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
//     mut structure_loaded_event_writer: EventWriter<StructureLoadedEvent>,
//     query: Query<&Structure>,
// ) {
//     for ev in event_reader.read() {
//         if let Ok(structure) = query.get(ev.0) {
//             for res in structure.all_chunks_iter(false) {
//                 // This will always be true because include_empty is false
//                 if let ChunkIteratorResult::FilledChunk {
//                     position: coords,
//                     chunk: _,
//                 } = res
//                 {
//                     chunk_set_event_writer.send(ChunkInitEvent {
//                         structure_entity: ev.0,
//                         coords,
//                     });
//                 }
//             }
//         }

//         structure_loaded_event_writer.send(StructureLoadedEvent { structure_entity: ev.0 });
//     }
// }

fn save_ships(query: Query<Entity, With<Ship>>, mut commands: Commands) {
    for ent in query.iter() {
        commands.entity(ent).insert(NeedsSaved);
    }
}

pub(super) fn register(app: &mut App) {
    app //.add_systems(PreUpdate, even_more_delayed_structure_event)
        //.add_systems(Update, delayed_structure_event)
        // .add_event::<DelayedStructureLoadEvent>()
        // .add_event::<EvenMoreDelayedStructureLoadEvent>()
        .add_systems(
            SAVING_SCHEDULE,
            (
                on_blueprint_structure.in_set(BlueprintingSystemSet::DoBlueprinting),
                on_save_structure.in_set(SavingSystemSet::DoSaving),
            ),
        )
        .add_systems(
            Update,
            (
                on_load_blueprint.in_set(LoadingBlueprintSystemSet::DoLoadingBlueprints),
                on_load_structure
                    .in_set(StructureLoadingSet::LoadStructure)
                    .in_set(LoadingSystemSet::DoLoading),
                save_ships.run_if(on_timer(Duration::from_secs(1))),
            ),
        );
}
