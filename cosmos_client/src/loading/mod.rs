//! Responsible for unloading far entities

use bevy::prelude::{
    App, Commands, CoreSet, DespawnRecursiveExt, Entity, IntoSystemConfig, Parent, Query, With,
    Without,
};
use cosmos_core::{
    entities::player::Player,
    physics::location::{Location, SECTOR_DIMENSIONS},
};

use crate::netty::flags::LocalPlayer;

const UNLOAD_DIST: f32 = SECTOR_DIMENSIONS * 10.0;

fn unload_far_entities(
    query: Query<(Entity, &Location), (Without<Player>, Without<Parent>)>,
    my_loc: Query<&Location, With<LocalPlayer>>,
    mut commands: Commands,
) {
    if let Ok(my_loc) = my_loc.get_single() {
        for (ent, loc) in query.iter() {
            if loc.distance_sqrd(my_loc) > UNLOAD_DIST * UNLOAD_DIST {
                println!("Unloading entity at {loc}!");
                commands.entity(ent).despawn_recursive();
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(unload_far_entities.in_base_set(CoreSet::First));
}
