//! Represents how a planet will be generated

use bevy::prelude::{App, Component, Entity};

pub mod grass_biosphere;
pub mod test_all_stone_biosphere;

/// This has to be redone.
pub trait TGenerateChunkEvent {
    /// Creates the generate chunk event
    fn new(x: usize, y: usize, z: usize, structure_entity: Entity) -> Self;
}

/// This has to be redone.
pub trait TBiosphere<T: Component, E: TGenerateChunkEvent> {
    /// Gets the marker component used to flag this planet's type
    fn get_marker_component(&self) -> T;
    /// Gets a component for this specific generate chunk event
    fn get_generate_chunk_event(&self, x: usize, y: usize, z: usize, structure_entity: Entity)
        -> E;
}

pub(super) fn register(app: &mut App) {
    grass_biosphere::register(app);
    test_all_stone_biosphere::register(app);
}
