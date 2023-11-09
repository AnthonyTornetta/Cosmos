//! Handles the saving/unloading of reactors

use bevy::prelude::{App, Commands, Entity, First, IntoSystemConfigs, Query, Update, With};
use cosmos_core::block::multiblock::reactor::Reactors;

use crate::persistence::{
    loading::{begin_loading, done_loading, NeedsLoaded},
    saving::{begin_saving, done_saving, NeedsSaved},
    SerializedData,
};

fn on_save_reactors(mut reactors_query: Query<(&Reactors, &mut SerializedData), With<NeedsSaved>>) {
    for (reactors, mut serialized_data) in reactors_query.iter_mut() {
        serialized_data.serialize_data("cosmos:reactors", reactors);
    }
}

fn on_load_reactors(mut commands: Commands, query: Query<(Entity, &SerializedData), With<NeedsLoaded>>) {
    for (entity, serialized_data) in query.iter() {
        let Some(reactors) = serialized_data.deserialize_data::<Reactors>("cosmos:reactors") else {
            continue;
        };

        commands.entity(entity).insert(reactors);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(First, on_save_reactors.after(begin_saving).before(done_saving))
        .add_systems(Update, on_load_reactors.after(begin_loading).before(done_loading));
}
