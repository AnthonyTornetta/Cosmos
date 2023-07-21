use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::structure::{
    events::StructureLoadedEvent,
    ship::{ship_builder::TShipBuilder, Ship},
    structure_iterator::ChunkIteratorResult,
    ChunkInitEvent, Structure,
};

use crate::persistence::{
    loading::{begin_loading, done_loading, NeedsLoaded},
    saving::{begin_saving, done_saving, NeedsSaved},
    SerializedData,
};

use super::server_ship_builder::ServerShipBuilder;

fn on_save_structure(mut query: Query<(&mut SerializedData, &Structure), (With<NeedsSaved>, With<Ship>)>) {
    for (mut s_data, structure) in query.iter_mut() {
        s_data.serialize_data("cosmos:structure", structure);
        s_data.serialize_data("cosmos:is_ship", &true);
    }
}

fn on_load_structure(
    query: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    mut event_writer: EventWriter<DelayedStructureLoadEvent>,
    mut commands: Commands,
) {
    for (entity, s_data) in query.iter() {
        if s_data.deserialize_data::<bool>("cosmos:is_ship").unwrap_or(false) {
            if let Some(mut structure) = s_data.deserialize_data::<Structure>("cosmos:structure") {
                let loc = s_data
                    .deserialize_data("cosmos:location")
                    .expect("Every ship should have a location when saved!");

                let mut entity_cmd = commands.entity(entity);

                let vel = s_data.deserialize_data("cosmos:velocity").unwrap_or(Velocity::zero());

                let builder = ServerShipBuilder::default();

                builder.insert_ship(&mut entity_cmd, loc, vel, &mut structure);

                let entity = entity_cmd.id();

                event_writer.send(DelayedStructureLoadEvent(entity));

                commands.entity(entity).insert(structure);
            }
        }
    }
}

/// I hate this, but the only way to prevent issues with events is to delay the sending of the chunk init events
/// by 2 frames, so two events are needed to do this. This is really horrible, but the only way I can think of
/// to get this to work ;(
#[derive(Debug, Event)]
struct DelayedStructureLoadEvent(pub Entity);
#[derive(Debug, Event)]
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
    mut structure_loaded_event_writer: EventWriter<StructureLoadedEvent>,
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

        structure_loaded_event_writer.send(StructureLoadedEvent { structure_entity: ev.0 });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PreUpdate, even_more_delayed_structure_event)
        // After to ensure 1 frame delay
        // Used to have `.after(even_more_delayed_structure_event)`
        .add_systems(Update, delayed_structure_event)
        .add_event::<DelayedStructureLoadEvent>()
        .add_event::<EvenMoreDelayedStructureLoadEvent>()
        .add_systems(First, on_save_structure.after(begin_saving).before(done_saving))
        .add_systems(Update, on_load_structure.after(begin_loading).before(done_loading));
}
