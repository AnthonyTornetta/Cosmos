//! Used to build stations

use bevy::{
    ecs::{schedule::IntoSystemConfigs, system::EntityCommands},
    prelude::{Added, App, Commands, Entity, Name, Query, Update},
};
use bevy_rapier3d::prelude::{ReadMassProperties, RigidBody, Velocity};

use crate::{
    persistence::{Blueprintable, LoadingDistance},
    physics::location::{CosmosBundleSet, Location},
    structure::{Structure, loading::StructureLoadingSet, structure_builder::TStructureBuilder},
};

use super::Station;

/// Distance (in sectors) a station should be loaded in
pub const STATION_LOAD_DISTANCE: u32 = 6;
/// Distance (in sectors) a station should be unloaded
pub const STATION_UNLOAD_DISTANCE: u32 = STATION_LOAD_DISTANCE + 1;

/// Implement this to add a custom way to build stations
pub trait TStationBuilder {
    /// Adds everything to the entity needed to have a station
    fn insert_station(&self, entity: &mut EntityCommands, location: Location, structure: &mut Structure);
}

/// Default way to build a station
pub struct StationBuilder<T: TStructureBuilder> {
    structure_builder: T,
}

impl<T: TStructureBuilder> StationBuilder<T> {
    /// Creates a station builder that uses the given structure builder
    pub fn new(structure_builder: T) -> Self {
        Self { structure_builder }
    }
}

impl<T: TStructureBuilder> TStationBuilder for StationBuilder<T> {
    fn insert_station(&self, entity: &mut EntityCommands, location: Location, structure: &mut Structure) {
        self.structure_builder
            .insert_structure(entity, location, Velocity::zero(), structure);

        entity.insert(Station);
    }
}

fn on_add_station(query: Query<(Entity, &Location), Added<Station>>, mut commands: Commands) {
    for (entity, loc) in query.iter() {
        commands.entity(entity).insert((
            RigidBody::Fixed,
            ReadMassProperties::default(),
            Blueprintable,
            LoadingDistance::new(STATION_LOAD_DISTANCE, STATION_UNLOAD_DISTANCE),
            Name::new(format!("Station @ {}", loc.sector())),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        on_add_station
            .after(CosmosBundleSet::HandleCosmosBundles)
            .in_set(StructureLoadingSet::AddStructureComponents),
    );
}
