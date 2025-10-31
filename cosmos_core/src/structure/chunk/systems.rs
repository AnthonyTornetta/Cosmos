use bevy::prelude::*;

use crate::{ecs::sets::FixedUpdateSet, structure::chunk::ChunkEntity};

/// Sometimes chunks slowly drift very tiny amounts due to floating point errors. This removes that
fn align_chunk_entity(mut q_chunk_trans: Query<&mut Transform, With<ChunkEntity>>) {
    for mut trans in q_chunk_trans.iter_mut() {
        if trans.translation.x != trans.translation.x.trunc() {
            trans.translation.x = trans.translation.x.round();
        }
        if trans.translation.y != trans.translation.y.trunc() {
            trans.translation.y = trans.translation.y.round();
        }
        if trans.translation.z != trans.translation.z.trunc() {
            trans.translation.z = trans.translation.z.round();
        }
        if trans.rotation != Quat::IDENTITY {
            trans.rotation = Quat::IDENTITY;
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        align_chunk_entity.in_set(FixedUpdateSet::PostLocationSyncingPostPhysics),
    );
}
