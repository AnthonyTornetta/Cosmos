use bevy::prelude::*;

fn sync_parent(q_changed_parent: Query<&Parent, Changed<Parent>>, removed_parent: RemovedComponents<Parent>) {}

pub(super) fn register(app: &mut App) {}
