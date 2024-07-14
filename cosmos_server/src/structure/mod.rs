//! Contains all structure-related information for the server

use bevy::{
    app::Update,
    prelude::{App, IntoSystemSetConfigs, SystemSet},
};

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
/// These systems don't run at any specific time.
/// Rather, they are used to remove ambiguities between different types of structures that will never share the same modifications.
///
/// Because we can't statically prove something with a "Ship" cannot have a "Station" component, using these systems is a way
/// to remove ambiguity errors from systems designed with this valid assumption.
pub enum StructureTypeSet {
    Ship,
    Planet,
    Station,
    Asteroid,
    /// Put systems in here that are ambiguous w/ a type of structure(s) but wouldn't in practice.
    None,
}

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

    use StructureTypeSet as S;

    app.configure_sets(
        Update,
        (
            S::Ship
                .ambiguous_with(S::Planet)
                .ambiguous_with(S::Station)
                .ambiguous_with(S::Asteroid)
                .ambiguous_with(S::None),
            S::Planet
                .ambiguous_with(S::Ship)
                .ambiguous_with(S::Station)
                .ambiguous_with(S::Asteroid)
                .ambiguous_with(S::None),
            S::Station
                .ambiguous_with(S::Ship)
                .ambiguous_with(S::Planet)
                .ambiguous_with(S::Asteroid)
                .ambiguous_with(S::None),
            S::Asteroid
                .ambiguous_with(S::Ship)
                .ambiguous_with(S::Planet)
                .ambiguous_with(S::Station)
                .ambiguous_with(S::None),
            S::None
                .ambiguous_with(S::Ship)
                .ambiguous_with(S::Planet)
                .ambiguous_with(S::Station)
                .ambiguous_with(S::Asteroid),
        ),
    );

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
