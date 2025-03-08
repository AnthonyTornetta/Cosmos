use bevy::prelude::*;
use cosmos_core::{entities::health::Dead, netty::system_sets::NetworkingSystemsSet};

fn on_add_dead(q_dead: Query<Entity, With<Dead>>) {
    for e in q_dead.iter() {
        info!("Dead: {e:?}");
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_add_dead.in_set(NetworkingSystemsSet::Between));
}
