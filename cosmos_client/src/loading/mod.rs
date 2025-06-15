//! Responsible for unloading far entities

use bevy::prelude::*;
use cosmos_core::{
    ecs::NeedsDespawned,
    entities::player::Player,
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    persistence::LoadingDistance,
    physics::location::{Location, LocationPhysicsSet, SectorUnit},
};

fn unload_far_entities(
    query: Query<(Entity, &Location, &LoadingDistance, Option<&Name>), (Without<Player>, Without<ChildOf>)>,
    my_loc: Query<&Location, With<LocalPlayer>>,
    mut commands: Commands,
) {
    if let Ok(my_loc) = my_loc.single() {
        for (ent, loc, unload_distance, name) in query.iter() {
            let ul_distance = unload_distance.unload_distance() as SectorUnit;

            if (loc.sector() - my_loc.sector()).abs().max_element() > ul_distance {
                info!("Unloading {ent:?} @ {loc} ({name:?}) - it's too far.");
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
