use bevy::prelude::{App, Component, IntoSystemConfigs, Query, With};
use cosmos_core::structure::chunk::Chunk;

use crate::persistence::{
    saving::{NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
    SerializedData,
};

#[derive(Component, Debug)]
pub struct SaveChunk(pub Chunk);

fn save_chunks(mut query: Query<(&mut SerializedData, &SaveChunk), With<NeedsSaved>>) {
    for (mut data, save_chunk) in query.iter_mut() {
        data.serialize_data("cosmos:chunk", &save_chunk.0);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(SAVING_SCHEDULE, save_chunks.in_set(SavingSystemSet::DoSaving));
}
