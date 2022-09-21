use bevy::prelude::{App, Entity};

pub struct StructureCreated {
    pub entity: Entity,
}

pub struct ChunkSetEvent {
    pub structure_entity: Entity,
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

pub fn register(app: &mut App) {
    app.add_event::<StructureCreated>()
        .add_event::<ChunkSetEvent>();
}
