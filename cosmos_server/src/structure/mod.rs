//! Contains all structure-related information for the server

use bevy::prelude::{App, IntoSystemSetConfigs, SystemSet};

use crate::persistence::{
    loading::{LoadingSystemSet, LOADING_SCHEDULE},
    saving::{BlueprintingSystemSet, SAVING_SCHEDULE},
};

pub mod asteroid;
pub mod block_health;
pub mod persistence;
pub mod planet;
pub mod server_structure_builder;
pub mod shared;
pub mod ship;
pub mod station;
pub mod systems;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum StructureTypesLoadingSystemSet {
    Ship,
    Planet,
    Station,
    Asteroid,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum StructureTypesBlueprintingSystemSet {
    Ship,
    Station,
    Asteroid,
}

pub(super) fn register(app: &mut App) {
    ship::register(app);
    systems::register(app);
    planet::register(app);
    block_health::register(app);
    asteroid::register(app);

    persistence::register(app);
    shared::register(app);
    station::register(app);

    use StructureTypesLoadingSystemSet as X;

    app.configure_sets(
        LOADING_SCHEDULE,
        (
            X::Ship
                .ambiguous_with(X::Planet)
                .ambiguous_with(X::Station)
                .ambiguous_with(X::Asteroid),
            X::Planet
                .ambiguous_with(X::Ship)
                .ambiguous_with(X::Station)
                .ambiguous_with(X::Asteroid),
            X::Station
                .ambiguous_with(X::Ship)
                .ambiguous_with(X::Planet)
                .ambiguous_with(X::Asteroid),
            X::Asteroid
                .ambiguous_with(X::Ship)
                .ambiguous_with(X::Planet)
                .ambiguous_with(X::Station),
        )
            .in_set(LoadingSystemSet::DoLoading),
    );

    use StructureTypesBlueprintingSystemSet as Y;

    app.configure_sets(
        SAVING_SCHEDULE,
        (
            Y::Ship.ambiguous_with(Y::Station).ambiguous_with(Y::Asteroid),
            Y::Station.ambiguous_with(Y::Ship).ambiguous_with(Y::Asteroid),
            Y::Asteroid.ambiguous_with(Y::Ship).ambiguous_with(Y::Station),
        )
            .in_set(BlueprintingSystemSet::DoBlueprinting),
    );
}
