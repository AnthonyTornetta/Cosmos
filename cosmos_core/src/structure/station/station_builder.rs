//! Used to build stations

use bevy::{
    ecs::{schedule::IntoSystemConfigs, system::EntityCommands},
    prelude::{Added, App, Commands, Entity, Name, Query, Update},
};
use bevy_rapier3d::prelude::{ReadMassProperties, RigidBody, Velocity};

use crate::{
    persistence::LoadingDistance,
    physics::location::Location,
    structure::{loading::StructureLoadingSet, structure_builder::TStructureBuilder, Structure},
};

use super::Station;

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

fn on_add_station(query: Query<Entity, Added<Station>>, mut commands: Commands) {
    for entity in query.iter() {
        commands.entity(entity).insert((
            RigidBody::Fixed,
            ReadMassProperties::default(),
            LoadingDistance::new(6, 7),
            Name::new("Station"),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_add_station.in_set(StructureLoadingSet::LoadStructure));
}
