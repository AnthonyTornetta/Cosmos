use bevy::prelude::*;

use crate::{ecs::sets::FixedUpdateSet, physics::location::Location};

// #[derive(Component, Debug)]
// pub struct WarpLoadPoint;

// #[derive(Component, Debug, Reflect)]
// pub struct WarpingTo(Location);

#[derive(Component)]
pub struct Warping;

#[derive(Component, Reflect)]
pub struct WarpTo {
    pub loc: Location,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum WarpingSet {
    StartWarping,
    DoneWarping,
}

pub(super) fn register(app: &mut App) {
    app.register_type::<WarpTo>();

    app.configure_sets(
        FixedUpdate,
        (WarpingSet::StartWarping, WarpingSet::DoneWarping)
            .chain()
            .in_set(FixedUpdateSet::Main),
    );
}
