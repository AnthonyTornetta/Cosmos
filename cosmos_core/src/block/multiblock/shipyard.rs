//! The shipyard multiblock logic

use crate::{
    block::{data::BlockData, multiblock::rectangle::RectangleMultiblockBounds},
    prelude::BlockCoordinate,
};
use bevy::{ecs::component::HookContext, prelude::*};

#[derive(Debug, Component, Reflect)]
/// A place used to assemble ships
pub struct Shipyard {
    controller: BlockCoordinate,
    bounds: RectangleMultiblockBounds,
}

impl Shipyard {
    /// Creates a new shipyard based on these conditions
    pub fn new(bounds: RectangleMultiblockBounds, controller: BlockCoordinate) -> Self {
        Self { bounds, controller }
    }

    /// Checks if this block coordinate is within the bounds of this shipyard (including the frame)
    pub fn coordinate_within(&self, coord: BlockCoordinate) -> bool {
        coord.within(self.bounds.negative_coords, self.bounds.positive_coords) || coord == self.controller
    }

    /// Returns the coordinate of this shipyard
    pub fn controller(&self) -> BlockCoordinate {
        self.controller
    }

    /// Returns the bounds of this shipyard (including frame)
    pub fn bounds(&self) -> RectangleMultiblockBounds {
        self.bounds
    }
}

#[derive(Debug, Component, Reflect)]
/// Contains a list of all [`Shipyard`]s this structure has
pub struct Shipyards(Vec<Entity>);

impl Shipyards {
    /// Iterates over all the [`Shipyard`]s this structure has
    pub fn iter(&self) -> impl Iterator<Item = Entity> {
        self.0.iter().copied()
    }
}

fn register_shipyard_component_hooks(world: &mut World) {
    world
        .register_component_hooks::<Shipyard>()
        .on_add(|mut world, HookContext { entity, .. }| {
            let Some(block_data) = world.get::<BlockData>(entity) else {
                error!("Shipyard missing block data!");
                return;
            };
            let structure = block_data.identifier.block.structure();
            if let Some(mut shipyards) = world.get_mut::<Shipyards>(structure) {
                shipyards.0.push(entity);
            } else {
                world.commands().entity(structure).insert(Shipyards(vec![entity]));
            }
        })
        .on_remove(|mut world, HookContext { entity, .. }| {
            let Some(block_data) = world.get::<BlockData>(entity) else {
                error!("Shipyard missing block data!");
                return;
            };
            let structure = block_data.identifier.block.structure();
            if let Some(mut shipyards) = world.get_mut::<Shipyards>(structure)
                && let Some((idx, _)) = shipyards.0.iter().enumerate().find(|x| *x.1 == entity)
            {
                shipyards.0.swap_remove(idx);
            }
        });
}

pub(super) fn register(app: &mut App) {
    app.register_type::<Shipyard>()
        .register_type::<Shipyards>()
        .add_systems(Startup, register_shipyard_component_hooks);
}
