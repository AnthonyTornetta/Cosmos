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
        println!("Saving planet!");
        s_data.serialize_data("cosmos:structure", structure);
        s_data.serialize_data("cosmos:is_planet", &true);
    }
}

fn on_load_structure(
    query: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
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

                    for res in structure.all_chunks_iter(false) {
                        // This will always be true because include_empty is false
                        if let ChunkIteratorResult::FilledChunk {
                            position: (x, y, z),
                            chunk: _,
                        } = res
                        {
                            chunk_set_event_writer.send(ChunkInitEvent {
                                structure_entity: entity,
                                x,
                                y,
                                z,
                            });
                        }
                    }

                    commands.entity(entity).insert(structure);

                    println!("Done loading planet!");
                }
            }
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system(on_save_structure.after(begin_saving).before(done_saving))
        .add_system(on_load_structure.after(begin_loading).before(done_loading));
}
