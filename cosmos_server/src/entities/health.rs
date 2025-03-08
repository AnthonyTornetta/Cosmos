use bevy::prelude::*;
use cosmos_core::{
    entities::health::{Dead, Health, HealthSet, MaxHealth},
    netty::system_sets::NetworkingSystemsSet,
};

use crate::persistence::make_persistent::{make_persistent, DefaultPersistentComponent};

impl DefaultPersistentComponent for Health {}
impl DefaultPersistentComponent for MaxHealth {}
impl DefaultPersistentComponent for Dead {}

fn on_change_health(mut commands: Commands, q_health: Query<(Entity, &Health), Changed<Health>>) {
    for (ent, hp) in q_health.iter() {
        if !hp.is_alive() {
            commands.entity(ent).insert(Dead);
        }
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<Health>(app);
    make_persistent::<MaxHealth>(app);
    make_persistent::<Dead>(app);

    app.add_systems(
        Update,
        on_change_health
            .in_set(HealthSet::ProcessHealthChange)
            .in_set(NetworkingSystemsSet::Between),
    );
}
