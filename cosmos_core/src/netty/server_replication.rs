//! Eventually used to replicate entities from the server -> client.
///
/// For now this is just for structure system replication
use bevy::ecs::entity::Entity;
use serde::{Deserialize, Serialize};

use crate::{
    block::gravity_well::UnderGravityWell,
    structure::systems::{StructureSystemId, StructureSystemTypeId},
};

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
    /// A gravity well. I'm trying stuff out rn, which is why this is a Vec<u8> instead of a gravity well
    GravityWell {
        /// The gravity well or None if this entity has no `UnderGravityWell` component.
        gravity_well: Option<UnderGravityWell>,
        /// The entity that has this component
        entity: Entity,
    },
}
