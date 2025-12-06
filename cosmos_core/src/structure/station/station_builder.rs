//! Used to build stations

use bevy::prelude::*;
use bevy_rapier3d::prelude::{ReadMassProperties, RigidBody};

use crate::{
    persistence::{Blueprintable, LoadingDistance},
    physics::location::Location,
    structure::loading::StructureLoadingSet,
};

use super::Station;

/// Distance (in sectors) a station should be loaded in
pub const STATION_LOAD_DISTANCE: u32 = 2;
/// Distance (in sectors) a station should be unloaded
pub const STATION_UNLOAD_DISTANCE: u32 = STATION_LOAD_DISTANCE + 1;

fn on_add_station(query: Query<(Entity, &Location), Or<(Added<Station>, (With<Station>, Added<Location>))>>, mut commands: Commands) {
    for (entity, loc) in query.iter() {
        commands.entity(entity).insert((
            RigidBody::KinematicPositionBased,
            ReadMassProperties::default(),
            Blueprintable,
            LoadingDistance::new(STATION_LOAD_DISTANCE, STATION_UNLOAD_DISTANCE),
            Name::new(format!("Station @ {}", loc.sector())),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(FixedUpdate, on_add_station.in_set(StructureLoadingSet::AddStructureComponents));
}
