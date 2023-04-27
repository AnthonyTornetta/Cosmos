use bevy::prelude::*;
use cosmos_core::{
    physics::{location::Location, player_world::PlayerWorld},
    structure::{
        planet::{planet_builder::TPlanetBuilder, Planet},
        Structure,
    },
};

use crate::{
    persistence::{
        loading::{begin_loading, done_loading, NeedsLoaded},
        saving::{begin_saving, done_saving, NeedsSaved},
        SerializedData,
    },
    structure::persistence::DelayedStructureLoadEvent,
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

fn on_load_structure(
    query: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    player_worlds: Query<&Location, With<PlayerWorld>>,
    mut event_writer: EventWriter<DelayedStructureLoadEvent>,
    mut commands: Commands,
) {
    for (entity, s_data) in query.iter() {
        if let Some(is_planet) = s_data.deserialize_data::<bool>("cosmos:is_planet") {
            if is_planet {
                if let Some(mut structure) =
                    s_data.deserialize_data::<Structure>("cosmos:structure")
                {
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

pub(super) fn register(app: &mut App) {
    app.add_system(on_save_structure.after(begin_saving).before(done_saving))
        .add_system(on_load_structure.after(begin_loading).before(done_loading));
}
