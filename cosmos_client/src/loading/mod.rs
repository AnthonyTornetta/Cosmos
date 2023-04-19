//! Responsible for unloading far entities

use bevy::prelude::{
    App, Commands, CoreSet, DespawnRecursiveExt, Entity, IntoSystemConfig, Parent, Query, With,
    Without,
};
use cosmos_core::{
    entities::player::Player, persistence::UnloadDistance, physics::location::Location,
};

use crate::netty::flags::LocalPlayer;

fn unload_far_entities(
    query: Query<(Entity, &Location, &UnloadDistance), (Without<Player>, Without<Parent>)>,
    my_loc: Query<&Location, With<LocalPlayer>>,
    mut commands: Commands,
) {
    if let Ok(my_loc) = my_loc.get_single() {
        for (ent, loc, unload_distance) in query.iter() {
            let ul_distance = unload_distance.unload_block_distance_squared();

            if loc.distance_sqrd(my_loc) > ul_distance {
                println!("Unloading entity at {loc}!");
                commands.entity(ent).despawn_recursive();
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(unload_far_entities.in_base_set(CoreSet::First));
}
