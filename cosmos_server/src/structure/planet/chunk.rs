use bevy::prelude::{App, Component, CoreSet, IntoSystemConfig, Query, With};
use cosmos_core::structure::chunk::Chunk;

use crate::persistence::{
    saving::{begin_saving, done_saving, NeedsSaved},
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
    app.add_system(
        save_chunks
            .in_base_set(CoreSet::First)
            .after(begin_saving)
            .before(done_saving),
    );
}
