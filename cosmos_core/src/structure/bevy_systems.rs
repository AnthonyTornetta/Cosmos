use bevy::prelude::*;

use crate::events::block_events::BlockDataSystemParams;

use super::Structure;

fn despawn_dead_block_data(mut bs_commands: BlockDataSystemParams, mut q_structures: Query<&mut Structure>) {
    for mut s in q_structures.iter_mut() {
        s.despawn_dead_block_data(&mut bs_commands);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PostUpdate, despawn_dead_block_data);
}