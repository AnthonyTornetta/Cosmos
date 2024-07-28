//! Responsible for unloading far entities

use bevy::prelude::{App, Commands, Entity, IntoSystemConfigs, Parent, Query, Update, With, Without};
use cosmos_core::{
    ecs::NeedsDespawned,
    entities::player::Player,
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    persistence::LoadingDistance,
    physics::location::{Location, LocationPhysicsSet, SectorUnit},
};

fn unload_far_entities(
    query: Query<(Entity, &Location, &LoadingDistance), (Without<Player>, Without<Parent>)>,
    my_loc: Query<&Location, With<LocalPlayer>>,
    mut commands: Commands,
) {
    if let Ok(my_loc) = my_loc.get_single() {
        for (ent, loc, unload_distance) in query.iter() {
            let ul_distance = unload_distance.unload_distance() as SectorUnit;

            if (loc.sector() - my_loc.sector()).abs().max_element() > ul_distance {
                commands.entity(ent).insert(NeedsDespawned);
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        unload_far_entities
            .in_set(NetworkingSystemsSet::Between)
            .after(LocationPhysicsSet::DoPhysics),
    );
}
