use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::structure::{
    ship::{ship_builder::TShipBuilder, Ship},
    Structure,
};

use crate::{
    persistence::{
        loading::{begin_loading, done_loading, NeedsLoaded},
        saving::{begin_saving, done_saving, NeedsSaved},
        SerializedData,
    },
    structure::persistence::DelayedStructureLoadEvent,
};

use super::server_ship_builder::ServerShipBuilder;

fn on_save_structure(
    mut query: Query<(&mut SerializedData, &Structure), (With<NeedsSaved>, With<Ship>)>,
) {
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
        if let Some(is_ship) = s_data.deserialize_data::<bool>("cosmos:is_ship") {
            if is_ship {
                if let Some(mut structure) =
                    s_data.deserialize_data::<Structure>("cosmos:structure")
                {
                    let mut entity_cmd = commands.entity(entity);
                    let loc = s_data
                        .deserialize_data("cosmos:location")
                        .expect("Every ship should have a location when saved!");

                    let vel = s_data
                        .deserialize_data("cosmos:velocity")
                        .unwrap_or(Velocity::zero());

                    let builder = ServerShipBuilder::default();

                    builder.insert_ship(&mut entity_cmd, loc, vel, &mut structure);

                    let entity = entity_cmd.id();

                    event_writer.send(DelayedStructureLoadEvent(entity));

                    commands.entity(entity).insert(structure);
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(on_save_structure.after(begin_saving).before(done_saving))
        .add_system(on_load_structure.after(begin_loading).before(done_loading));
}
