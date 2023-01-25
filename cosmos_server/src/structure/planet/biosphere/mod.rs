use bevy::prelude::{App, Component, Entity};

pub mod grass_biosphere;
pub mod test_all_stone_biosphere;

pub trait TGenerateChunkEvent {
    fn new(x: usize, y: usize, z: usize, structure_entity: Entity) -> Self;
}

pub trait TBiosphere<T: Component, E: TGenerateChunkEvent> {
    fn get_marker_component(&self) -> T;
    fn get_generate_chunk_event(&self, x: usize, y: usize, z: usize, structure_entity: Entity)
        -> E;
}

pub(crate) fn register(app: &mut App) {
    grass_biosphere::register(app);
    test_all_stone_biosphere::register(app);
}
