use bevy::prelude::{Component, Entity};

pub trait TGenerateChunkEvent {
    fn new(x: usize, y: usize, z: usize, structure_entity: Entity) -> Self;
}

pub trait TBiosphere<T: Component, E: TGenerateChunkEvent> {
    fn get_marker_component(&self) -> T;
    fn get_generate_chunk_event(&self, x: usize, y: usize, z: usize, structure_entity: Entity)
        -> E;
}
