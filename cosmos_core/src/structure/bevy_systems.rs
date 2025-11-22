use bevy::prelude::*;

use crate::block::data::BlockData;

use super::Structure;

fn despawn_dead_block_data(mut commands: Commands, mut q_block_data: Query<&mut BlockData>, mut q_structures: Query<&mut Structure>) {
    for mut s in q_structures.iter_mut() {
        s.despawn_dead_block_data(&mut q_block_data, &mut commands);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PostUpdate, despawn_dead_block_data);
}
