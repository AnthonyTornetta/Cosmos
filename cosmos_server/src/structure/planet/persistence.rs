use bevy::prelude::*;
use cosmos_core::structure::{
    loading::ChunksNeedLoaded,
    planet::{planet_builder::TPlanetBuilder, Planet},
    structure_iterator::ChunkIteratorResult,
    ChunkInitEvent, Structure,
};

use crate::persistence::{
    loading::{begin_loading, done_loading, NeedsLoaded},
    saving::{begin_saving, done_saving, NeedsSaved},
    SerializedData,
};

use super::server_planet_builder::ServerPlanetBuilder;

fn on_save_structure(
    mut query: Query<(&mut SerializedData, &Structure), (With<NeedsSaved>, With<Planet>)>,
) {
    for (mut s_data, structure) in query.iter_mut() {
        s_data.serialize_data("cosmos:structure", structure);
        s_data.serialize_data("cosmos:is_planet", &true);
    }
}

// I hate this

// The only way to prevent issues with events is to delay the sending of the chunk init events by 2 frames,
// so two events are needed to do this. This is really horrible, but the only way I can think of
// to get this to work ;(
struct DelayedStructureLoadEvent(Entity);
struct EvenMoreDelayedStructureLoadEvent(Entity);

fn delayed_structure_event(
    mut event_reader: EventReader<DelayedStructureLoadEvent>,
    mut event_writer: EventWriter<EvenMoreDelayedStructureLoadEvent>,
) {
    for ev in event_reader.iter() {
        event_writer.send(EvenMoreDelayedStructureLoadEvent(ev.0));
    }
}

fn even_more_delayed_structure_event(
    mut event_reader: EventReader<EvenMoreDelayedStructureLoadEvent>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
    query: Query<&Structure>,
) {
    for ev in event_reader.iter() {
        if let Ok(structure) = query.get(ev.0) {
            for res in structure.all_chunks_iter(false) {
                // This will always be true because include_empty is false
                if let ChunkIteratorResult::FilledChunk {
                    position: (x, y, z),
                    chunk: _,
                } = res
                {
                    chunk_set_event_writer.send(ChunkInitEvent {
                        structure_entity: ev.0,
                        x,
                        y,
                        z,
                    });
                }
            }
        }
    }
}

fn on_load_structure(
    query: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    mut event_writer: EventWriter<DelayedStructureLoadEvent>,
    mut commands: Commands,
) {
    for (entity, s_data) in query.iter() {
        if let Some(is_planet) = s_data.deserialize_data::<bool>("cosmos:is_planet") {
            if is_planet {
                if let Some(mut structure) =
                    s_data.deserialize_data::<Structure>("cosmos:structure")
                {
                    let mut entity_cmd = commands.entity(entity);

                    let builder = ServerPlanetBuilder::default();
                    let loc = s_data
                        .deserialize_data("cosmos:location")
                        .expect("Every planet should have a location when saved!");

                    builder.insert_planet(&mut entity_cmd, loc, &mut structure);

                    let entity = entity_cmd.id();

                    entity_cmd.insert(ChunksNeedLoaded {
                        amount_needed: structure.all_chunks_iter(false).len(),
                    });

                    event_writer.send(DelayedStructureLoadEvent(entity));

                    commands.entity(entity).insert(structure);
                }
            }
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system(on_save_structure.after(begin_saving).before(done_saving))
        .add_system(on_load_structure.after(begin_loading).before(done_loading))
        .add_system(even_more_delayed_structure_event.in_base_set(CoreSet::PreUpdate))
        // After to ensure 1 frame delay
        .add_system(delayed_structure_event.after(even_more_delayed_structure_event))
        .add_event::<DelayedStructureLoadEvent>()
        .add_event::<EvenMoreDelayedStructureLoadEvent>();
}
