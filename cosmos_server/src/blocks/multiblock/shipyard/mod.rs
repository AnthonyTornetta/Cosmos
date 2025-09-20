use bevy::{ecs::relationship::Relationship, prelude::*};
use cosmos_core::prelude::BlockCoordinate;

use crate::blocks::multiblock::checker::rectangle::RectangleMultiblockBounds;

mod impls;

#[derive(Debug, Component, Reflect)]
struct Shipyard {
    controller: BlockCoordinate,
    bounds: RectangleMultiblockBounds,
}

impl Shipyard {
    pub fn coordinate_within(&self, coord: BlockCoordinate) -> bool {
        coord.within(self.bounds.negative_coords, self.bounds.positive_coords) || coord == self.controller
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
