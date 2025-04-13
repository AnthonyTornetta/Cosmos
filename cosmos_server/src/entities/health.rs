use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use cosmos_core::{
    entities::health::{Dead, Health, HealthSet, MaxHealth},
    netty::system_sets::NetworkingSystemsSet, structure::ship::pilot::Pilot,
};

use crate::persistence::make_persistent::{make_persistent, DefaultPersistentComponent};

impl DefaultPersistentComponent for Health {}
impl DefaultPersistentComponent for MaxHealth {}
impl DefaultPersistentComponent for Dead {}

fn on_change_health(mut commands: Commands, q_health: Query<(Entity, &Health), Changed<Health>>) {
    for (ent, hp) in q_health.iter() {
        if !hp.is_alive() {
            commands.entity(ent).insert(Dead).remove_parent_in_place().remove::<Pilot>;
        }
    }
}

fn regenerate_health(mut q_health: Query<(&mut Health, &MaxHealth), Without<Dead>>) {
    for (mut hp, max_hp) in q_health.iter_mut() {
        hp.heal(1, max_hp);
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<Health>(app);
    make_persistent::<MaxHealth>(app);
    make_persistent::<Dead>(app);

    app.add_systems(
        Update,
        (regenerate_health.run_if(on_timer(Duration::from_secs(10))), on_change_health)
            .in_set(HealthSet::ProcessHealthChange)
            .in_set(NetworkingSystemsSet::Between),
    );
}
