use bevy::{ecs::relationship::Relationship, prelude::*};
use cosmos_core::prelude::BlockCoordinate;

mod impls;

#[derive(Debug, Reflect)]
struct ShipyardBounds {
    min: BlockCoordinate,
    max: BlockCoordinate,
}

#[derive(Debug, Component, Reflect)]
struct Shipyard {
    controller: BlockCoordinate,
    bounds: ShipyardBounds,
}

impl Shipyard {
    pub fn coordinate_within(&self, coord: BlockCoordinate) -> bool {
        coord.within(self.bounds.min, self.bounds.max) || coord == self.controller
    }
}

#[derive(Debug, Component, Reflect)]
pub struct Shipyards(Vec<Entity>);

impl Shipyards {
    pub fn iter(&self) -> impl Iterator<Item = Entity> {
        self.0.iter().copied()
    }
}

pub(super) fn register(app: &mut App) {
    impls::register(app);

    app.register_type::<Shipyard>().register_type::<Shipyards>();
}
