//! Eventually used to replicate entities from the server -> client.
///
/// For now this is just for structure system replication
use bevy::ecs::entity::Entity;
use serde::{Deserialize, Serialize};

use crate::structure::systems::{StructureSystemId, StructureSystemTypeId};

#[derive(Debug, Serialize, Deserialize)]
/// Eventually used to replicate entities from the server -> client.
///
/// For now this is just for structure system replication
pub enum ReplicationMessage {
    /// Replicates a structure system
    SystemReplication {
        /// The structure entity
        structure_entity: Entity,
        /// The system's id (unique to this structure)
        system_id: StructureSystemId,
        /// The type of the structure system being sent over
        system_type_id: StructureSystemTypeId,
        /// The serialized data of this structure system (serialized via `cosmos_encoder::serialize`).
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
