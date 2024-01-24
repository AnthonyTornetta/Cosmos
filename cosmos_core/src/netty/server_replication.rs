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
    /// Sent whenever the activness of a structure system changes
    SystemStatus {
        /// The structure system id
        system_id: StructureSystemId,
        /// The structure that contains this system
        structure_entity: Entity,
        /// If the system is active or not
        active: bool,
    },
}
