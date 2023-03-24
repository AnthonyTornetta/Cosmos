use bevy::prelude::*;
use cosmos_core::structure::{planet::Planet, Structure};

use crate::persistence::{
    loading::{begin_loading, done_loading, NeedsLoaded},
    saving::{begin_saving, done_saving, NeedsSaved},
    SerializedData,
};

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
    mut commands: Commands,
) {
    for (entity, s_data) in query.iter() {
        if let Some(is_planet) = s_data.deserialize_data::<bool>("cosmos:is_planet") {
            if is_planet {
                if let Some(structure) = s_data.deserialize_data::<Structure>("cosmos:structure") {
                    commands.entity(entity).insert(structure);
                }
            }
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system(on_save_structure.after(begin_saving).before(done_saving))
        .add_system(on_load_structure.after(begin_loading).before(done_loading));
}
