use bevy::prelude::{App, Entity};

use super::{structure_iterator::BlockIterator, Structure};

pub struct StructureCreated {
    pub entity: Entity,
}

/// This will be created once all chunks have been populated
pub struct StructureLoadedEvent {
    pub structure_entity: Entity,
}

/// This should only be used to initially setup a structure.
/// Do **not** overwrite existing blocks with this.
/// Some systems will get out of sync if you misuse this.
/// Params:
/// - structure_entity: The entity of the structure this is a part of
/// - x | Chunk's coordinate in the structure
/// - y | Chunk's coordinate in the structure
/// - z | Chunk's coordinate in the structure
#[derive(Debug)]
pub struct ChunkSetEvent {
    /// The entity of the structure this is a part of
    pub structure_entity: Entity,
    /// Chunk's coordinate in the structure
    pub x: usize,
    /// Chunk's coordinate in the structure    
    pub y: usize,
    /// Chunk's coordinate in the structure    
    pub z: usize,
}

impl ChunkSetEvent {
    pub fn iter_blocks<'a>(&'a self, structure: &'a Structure, include_air: bool) -> BlockIterator {
        structure.block_iter_for_chunk((self.x, self.y, self.z), include_air)
    }
}

pub fn register(app: &mut App) {
    app.add_event::<StructureCreated>()
        .add_event::<ChunkSetEvent>()
        .add_event::<StructureLoadedEvent>();
}
