use bevy::ecs::entity::Entity;
use serde::{Deserialize, Serialize};

use crate::structure::systems::{StructureSystemId, StructureSystemTypeId};

#[derive(Debug, Serialize, Deserialize)]
pub enum ReplicationMessage {
    SystemReplication {
        structure_entity: Entity,
        system_id: StructureSystemId,
        system_type_id: StructureSystemTypeId,
        raw: Vec<u8>,
    },
}
